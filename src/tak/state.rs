//
// This file is part of Takkerus.
//
// Takkerus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Takkerus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Takkerus. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright 2016 Chris Foster
//

use std::fmt::{self, Write};

use tak::{Color, GameError, Piece, Ply, Seat, StateAnalysis, Win};
use tak::state_analysis::{BOARD, EDGE, Bitmap};

#[derive(Clone, Debug)]
pub struct State {
    pub p1: Seat,
    pub p2: Seat,

    pub board: Vec<Vec<Vec<Piece>>>,

    pub ply_count: u16,
    pub analysis: StateAnalysis,
}

impl State {
    pub fn new(board_size: usize) -> State {
        let (flatstone_count, capstone_count) = match board_size {
            3 => (10, 0),
            4 => (15, 0),
            5 => (21, 1),
            6 => (30, 1),
            7 => (40, 1),
            8 => (50, 2),
            _ => panic!("Illegal board size!"),
        };

        State {
            p1: Seat::new(Color::White, flatstone_count, capstone_count),
            p2: Seat::new(Color::Black, flatstone_count, capstone_count),
            board: vec![vec![Vec::new(); board_size]; board_size],
            ply_count: 0,
            analysis: StateAnalysis::new(board_size),
        }
    }

    pub fn execute_ply(&self, ply: &Ply) -> Result<State, GameError> {
        let mut next = self.clone();
        next.ply_count += 1;

        let board_size = next.board.len();

        match ply {
            &Ply::Place { x, y, ref piece } => {
                if !next.board[x][y].is_empty() {
                    return Err(GameError::IllegalPlacement);
                }

                let count = match piece {
                    &Piece::Flatstone(color) |
                    &Piece::StandingStone(color) => if color == Color::White {
                        &mut next.p1.flatstone_count
                    } else {
                        &mut next.p2.flatstone_count
                    },
                    &Piece::Capstone(color) => if color == Color::White {
                        &mut next.p1.capstone_count
                    } else {
                        &mut next.p2.capstone_count
                    },
                };

                if *count > 0 {
                    *count -= 1;
                } else {
                    return Err(GameError::InsufficientPieces);
                }

                next.board[x][y].push(piece.clone());

                match piece {
                    &Piece::Flatstone(color) => next.analysis.add_flatstone(
                        color, x, y, next.board[x][y].len() - 1,
                    ),
                    block => next.analysis.add_blocking_stone(block, x, y),
                }

                match piece {
                    &Piece::Flatstone(_) |
                    &Piece::Capstone(_) => next.analysis.calculate_road_groups(),
                    _ => (),
                }
            },
            &Ply::Slide { x, y, direction, ref drops } => {
                let grab = drops.iter().fold(0, |acc, x| acc + x) as usize;

                if grab > board_size || next.board[x][y].is_empty() {
                    return Err(GameError::IllegalSlide);
                }

                let mut stack = Vec::new();
	            for _ in 0..grab {
	                let piece = next.board[x][y].pop().unwrap();

	                match piece {
	                    Piece::Flatstone(color) => next.analysis.remove_flatstone(
	                        color, x, y, next.board[x][y].len(),
                        ),
                        ref block => next.analysis.remove_blocking_stone(block, x, y),
                    }

                    match next.board[x][y].last() {
                        Some(revealed) => next.analysis.reveal_flatstone(
                            revealed.get_color(), x, y,
                        ),
                        None => (),
                    }

	                stack.push(piece);
                }

                let (dx, dy) = direction.to_offset();

                let mut nx = x as i8;
                let mut ny = y as i8;
                for drop in drops {
                    nx += dx;
                    ny += dy;

                    if nx < 0 || nx >= board_size as i8 ||
                       ny < 0 || ny >= board_size as i8 {
                        return Err(GameError::OutOfBounds);
                    }

                    if !next.board[nx as usize][ny as usize].is_empty() {
                        let target_top = next.board[nx as usize][ny as usize].last().unwrap().clone();
                        match target_top {
                            Piece::Capstone(_) => return Err(GameError::IllegalSlide),
                            Piece::StandingStone(color) => if stack.len() == 1 {
                                match stack[0] {
                                    Piece::Capstone(_) => {
                                        *next.board[nx as usize][ny as usize].last_mut().unwrap() = Piece::Flatstone(color);
                                        next.analysis.remove_blocking_stone(&Piece::StandingStone(color), nx as usize, ny as usize);
                                        next.analysis.add_flatstone(
                                            color, nx as usize, ny as usize,
                                            next.board[nx as usize][ny as usize].len() - 1,
                                        )
                                    },
                                    _ => return Err(GameError::IllegalSlide),
                                }
                            } else {
                                return Err(GameError::IllegalSlide);
                            },
                            _ => (),
                        }
                    }

                    for _ in 0..*drop {
                        match next.board[nx as usize][ny as usize].last() {
                            Some(covered) => next.analysis.cover_flatstone(
                                covered.get_color(), nx as usize, ny as usize,
                            ),
                            None => (),
                        }

                        let piece = stack.pop().unwrap();

                        match piece {
                            Piece::Flatstone(color) => next.analysis.add_flatstone(
                                color, nx as usize, ny as usize,
                                next.board[nx as usize][ny as usize].len(),
                            ),
                            ref block => next.analysis.add_blocking_stone(
                                block, nx as usize, ny as usize,
                            ),
                        }

                        next.board[nx as usize][ny as usize].push(piece);
                    }
                }

                next.analysis.calculate_road_groups();
            },
        }

        Ok(next)
    }

    pub fn check_win(&self) -> Win {
        let board_size = self.board.len();
        let a = &self.analysis;

        let has_road = |groups: &Vec<Bitmap>| {
            use tak::Direction::*;

            for group in groups.iter() {
                if (group & EDGE[board_size][North as usize] != 0 &&
                    group & EDGE[board_size][South as usize] != 0) ||
                   (group & EDGE[board_size][West as usize] != 0 &&
                    group & EDGE[board_size][East as usize] != 0) {
                    return true;
                }
            }

            false
        };

        let p1_has_road = has_road(&a.p1_road_groups);
        let p2_has_road = has_road(&a.p2_road_groups);

        if p1_has_road && p2_has_road {
            if self.ply_count % 2 == 1 {
                Win::Road(Color::White)
            } else {
                Win::Road(Color::Black)
            }
        } else if p1_has_road {
            Win::Road(Color::White)
        } else if p2_has_road {
            Win::Road(Color::Black)
        } else if (self.p1.flatstone_count + self.p1.capstone_count) == 0 ||
                  (self.p2.flatstone_count + self.p2.capstone_count) == 0 ||
                  (a.p1_pieces | a.p2_pieces) == BOARD[board_size] {
            if a.p1_flatstone_count > a.p2_flatstone_count {
                Win::Flat(Color::White)
            } else if a.p2_flatstone_count > a.p1_flatstone_count {
                Win::Flat(Color::Black)
            } else {
                Win::Draw
            }
        } else {
            Win::None
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let board_size = self.board.len();

        let column_widths = self.board.iter().map(|column| {
            column.iter().fold(6, |max, stack| {
                let stack_width = stack.iter().fold(0, |acc, piece| {
                    match piece {
                        &Piece::Flatstone(_) => acc + 1,
                        _ => acc + 2,
                    }
                }) + 3 + if !stack.is_empty() {
                    stack.len() - 1
                } else {
                    0
                };

                if max > stack_width { max } else { stack_width }
            })
        }).collect::<Vec<_>>();

        write!(f, "\n Player 1: {:>2} flatstone{}", self.p1.flatstone_count,
            if self.p1.flatstone_count != 1 { "s" } else { "" }
        ).ok();

        if self.p1.capstone_count > 0 {
            write!(f, ", {} capstone{}", self.p1.capstone_count,
                if self.p1.capstone_count != 1 { "s" } else { "" }
            ).ok();
        }

        write!(f, "\n Player 2: {:>2} flatstone{}", self.p2.flatstone_count,
            if self.p2.flatstone_count != 1 { "s" } else { "" }
        ).ok();

        if self.p2.capstone_count > 0 {
            write!(f, ", {} capstone{}\n\n", self.p2.capstone_count,
                if self.p2.capstone_count != 1 { "s" } else { "" }
            ).ok();
        } else {
            write!(f, "\n\n").ok();
        }

        for row in (0..board_size).rev() {
            write!(f, " {}   ", row + 1).ok();

            for column in 0..board_size {
                let mut c = String::new();
                write!(c, "[").ok();

                for (index, piece) in self.board[column][row].iter().rev().enumerate() {
                    if index > 0 {
                        write!(c, " ").ok();
                    }

                    write!(c, "{}", match piece.get_color() {
                        Color::White => "W",
                        Color::Black => "B",
                    }).ok();

                    match piece {
                        &Piece::StandingStone(_) => { write!(c, "S").ok(); },
                        &Piece::Capstone(_) => { write!(c, "C").ok(); },
                        _ => (),
                    }
                }

                write!(c, "]").ok();

                write!(f, "{:<width$}", c, width = column_widths[column]).ok();
            }

            write!(f, "\n").ok();
        }

        write!(f, "\n     ").ok();

        for (index, column_width) in column_widths.iter().enumerate() {
            write!(f, "{:<width$}", (index as u8 + 97) as char, width = column_width).ok();
        }

        write!(f, "\n")
    }
}
