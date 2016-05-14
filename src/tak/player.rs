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

use tak::Color;

#[derive(Clone, Debug)]
pub struct Player {
    pub color: Color,
    pub flatstone_count: u8,
    pub capstone_count: u8,
}

impl Player {
    pub fn new(color: Color, flatstone_count: u8, capstone_count: u8) -> Player {
        Player {
            color: color,
            flatstone_count: flatstone_count,
            capstone_count: capstone_count,
        }
    }
}
