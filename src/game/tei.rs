use crate::game::{Message, State};
use crate::player::{Player, PvSearchPlayer};
use crate::Color;
use crate::Ply;
use std::any::Any;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::thread;

struct DummyPlayer {}

impl Player for DummyPlayer {
    fn initialize(
        &mut self,
        to_game: Sender<(Color, super::Message)>,
        opponent: &dyn Player,
    ) -> Result<Sender<super::Message>, String> {
        unimplemented!()
    }
    fn get_name(&self) -> String {
        "Dummy Player".to_string()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
pub enum TeiCommand {
    Stop,
    Quit,
    Go(String),
    Position(String),
    NewGame(usize),
}

struct TimeLeft {
    wtime: u64,
    btime: u64,
    winc: u64,
    binc: u64,
}

impl TimeLeft {
    pub fn new(tei_str: &str) -> Self {
        let mut ret = Self {
            wtime: 1000,
            btime: 1000,
            winc: 0,
            binc: 0,
        };
        for (field, val) in tei_str
            .split_whitespace()
            .zip(tei_str.split_whitespace().skip(1))
        {
            match (field, val.parse()) {
                ("wtime", Ok(val)) => ret.wtime = val,
                ("btime", Ok(val)) => ret.btime = val,
                ("winc", Ok(val)) => ret.winc = val,
                ("binc", Ok(val)) => ret.binc = val,
                _ => {}
            }
        }
        ret
    }
    fn use_time(&self, est_plies: usize, side_to_move: Color) -> u64 {
        let (time_bank, inc) = match side_to_move {
            Color::White => (self.wtime, self.winc),
            Color::Black => (self.btime, self.binc),
        };
        let use_bank = time_bank / (est_plies + 2) as u64 / 1000;
        use_bank + inc / 1000
    }
}

pub fn play_game_tei(tei_receiver: Receiver<TeiCommand>) -> Result<(), RecvError> {
    let (to_game, from_engine) = channel();
    let mut player = PvSearchPlayer::with_goal(12);
    let to_engine = player
        .initialize(to_game, &DummyPlayer {})
        .expect("Failed to init player");
    let mut board = None;
    let mut size = 5;
    loop {
        let message = tei_receiver.recv()?;
        match message {
            TeiCommand::NewGame(s) => {
                board = Some(State::new(s));
                size = s;
                // Todo figure out if color matters
                to_engine.send(Message::GameStart(Color::White)).unwrap();
            }
            TeiCommand::Go(s) => {
                // Todo parse time
                let go_state = board.take().unwrap();
                to_engine.send(Message::MoveRequest(go_state)).unwrap();
                let (_color, message) = from_engine.recv()?;

                if let Message::MoveResponse(ply) = message {
                    // println!("info {}", outcome);
                    println!("bestmove {}", ply);
                } else {
                    println!("Something went wrong, search failed!");
                }
            }
            TeiCommand::Position(s) => {
                let mut side_to_move = Color::White;
                let mut ply_count = 0;
                let mut plies = Vec::new();
                for m in s.split_whitespace() {
                    // Swap colors in opening
                    let color = if ply_count < 2 {
                        side_to_move.flip()
                    } else {
                        side_to_move
                    };
                    if let Some(m) = Ply::from_ptn(m, color) {
                        plies.push(m);
                        ply_count += 1;
                        side_to_move = side_to_move.flip();
                    }
                }
                board = Some(State::from_plies(size, &plies).expect("Could not parse ptn!"));
            }
            TeiCommand::Quit => {
                break;
            }
            _ => println!("Unknown command: {:?}", message),
        }
    }
    Ok(())
}

pub fn identify() {
    println!("id name Takkerus");
    println!("id author Chris Foster");
    println!("teiok");
}

pub fn tei_loop(sender: Sender<TeiCommand>) {
    thread::spawn(move || {
        let mut buffer = String::new();
        loop {
            std::io::stdin()
                .read_line(&mut buffer)
                .expect("Could not read line");
            let line = buffer.trim();
            if line == "tei" {
                identify();
            } else if line == "isready" {
                println!("readyok");
            } else if line == "quit" {
                sender.send(TeiCommand::Quit).unwrap();
                break;
            } else if line == "stop" {
                sender.send(TeiCommand::Stop).unwrap();
            } else if line.starts_with("position") {
                sender.send(TeiCommand::Position(line.to_string())).unwrap();
            } else if line.starts_with("go") {
                sender.send(TeiCommand::Go(line.to_string())).unwrap();
            } else if line.starts_with("teinewgame") {
                let size = line.split_whitespace().find_map(|x| x.parse().ok());
                sender.send(TeiCommand::NewGame(size.unwrap())).unwrap();
            } else {
                println!("Unknown Tei Command: {}", buffer);
            }
            buffer.clear();
        }
    });
}
