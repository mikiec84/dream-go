// Copyright 2019 Karl Sundequist Blomdahl <karl.sundequist.blomdahl@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use asm::count_zeros;
use board_fast::*;
use board::Board;
use color::Color;
use dg_utils::{max, min};

use super::ladder::Ladder;
use super::symmetry;

/// The number of features that the board will provide.
pub const NUM_FEATURES: usize = 40;

/// The total size (in elements) of the feature set.
pub const FEATURE_SIZE: usize = NUM_FEATURES * 361;

/// Utility function for determining the data format of the array returned by
/// `get_features`.
pub trait Order {
    fn index(c: usize, i: usize) -> usize;
}

/// Implementation of `Order` for the data format `NCHW`.
pub struct CHW;

impl Order for CHW {
    fn index(c: usize, i: usize) -> usize {
        c * 361 + i
    }
}

/// Implementation of `Order` for the data format `NHWC`.
pub struct HWC;

impl Order for HWC {
    fn index(c: usize, i: usize) -> usize { NUM_FEATURES * i + c }
}

pub trait Features {
    /// Returns the features of the current object in the given order and data
    /// type.
    ///
    /// # Arguments
    ///
    /// * `to_move` - the color of the current player
    /// * `symmetry` - the symmetry to use
    ///
    fn get_features<O: Order, T: From<f32> + Copy>(
        &self,
        to_move: Color,
        symmetry: symmetry::Transform
    ) -> Vec<T>;
}

impl Features for Board {
    /// Returns the features of the current board state for the given color,
    /// it returns the following features. Divided into four sections based
    /// on their intended purpose (regardless of what the network does with
    /// them).
    /// 
    /// ## Global properties
    /// 
    ///  1. A constant plane filled with ones if we are black
    ///  2. A constant plane filled with ones if we are white
    ///  3. A constant plane filled with ones if any move is super-ko
    /// 
    /// ## One-hot historic board state
    /// 
    ///  4. Most recent move ( 0)
    ///  5. Most recent move (-1)
    /// 
    /// ## Liberties
    /// 
    ///  6. Our liberties (>= 1)
    ///  7. Our liberties (>= 2)
    ///  8. Our liberties (>= 3)
    ///  9. Our liberties (>= 4)
    /// 10. Our liberties (>= 5)
    /// 11. Our liberties (>= 6)
    /// 12. Our liberties (>= 7)
    /// 13. Our liberties (>= 8)
    /// 14. Our liberties after move (>= 1)
    /// 15. Our liberties after move (>= 2)
    /// 16. Our liberties after move (>= 3)
    /// 17. Our liberties after move (>= 4)
    /// 18. Our liberties after move (>= 5)
    /// 19. Our liberties after move (>= 6)
    /// 20. Our liberties after move (>= 7)
    /// 21. Our liberties after move (>= 8)
    /// 22. Opponent liberties (>= 1)
    /// 23. Opponent liberties (>= 2)
    /// 24. Opponent liberties (>= 3)
    /// 25. Opponent liberties (>= 4)
    /// 26. Opponent liberties (>= 5)
    /// 27. Opponent liberties (>= 6)
    /// 28. Opponent liberties (>= 7)
    /// 29. Opponent liberties (>= 8)
    /// 30. Opponent liberties after move (>= 1)
    /// 31. Opponent liberties after move (>= 2)
    /// 32. Opponent liberties after move (>= 3)
    /// 33. Opponent liberties after move (>= 4)
    /// 34. Opponent liberties after move (>= 5)
    /// 35. Opponent liberties after move (>= 6)
    /// 36. Opponent liberties after move (>= 7)
    /// 37. Opponent liberties after move (>= 8)
    ///
    /// ## Vertex properties
    /// 
    /// 38. Is super-ko
    /// 39. Is ladder capture
    /// 40. Is ladder escape
    ///
    /// # Arguments
    ///
    /// * `to_move` - the color of the current player
    /// * `symmetry` - the symmetry to extract the features to
    ///
    fn get_features<O: Order, T: From<f32> + Copy>(
        &self,
        to_move: Color,
        symmetry: symmetry::Transform
    ) -> Vec<T>
    {
        let c_0 = T::from(0.0);
        let c_1 = T::from(1.0);

        let mut features = vec! [c_0; FEATURE_SIZE];
        let symmetry_table = symmetry.get_table();
        let current = to_move as u8;
        let opponent = to_move.opposite();

        // board state (one-hot historic)
        for (i, index) in self.history.iter().take(2).enumerate() {
            if index == 361 {
                // pass
            } else {
                let other = symmetry_table[index as usize] as usize;

                features[O::index(3+i, other)] = c_1;
            }
        }

        // liberties
        let mut liberties = [0; 368];

        for index in 0..361 {
            let other = symmetry_table[index] as usize;

            if self.inner.vertices[index].color() != 0 {
                let start = if self.inner.vertices[index].color() == current { 5 } else { 21 };
                let num_liberties = ::std::cmp::min(
                    get_num_liberties(&self.inner, index, &mut liberties),
                    8
                );

                for i in 0..num_liberties {
                    features[O::index(start+i, other)] = c_1;
                }
            } else {
                if _is_valid_memoize(&self.inner, to_move, index, &mut liberties) {
                    let num_liberties = ::std::cmp::min(
                        get_num_liberties_if(&self.inner, to_move, index, &mut liberties),
                        8
                    );

                    for i in 0..num_liberties {
                        features[O::index(13+i, other)] = c_1;
                    }
                }

                if _is_valid_memoize(&self.inner, opponent, index, &mut liberties) {
                    let num_liberties = ::std::cmp::min(
                        get_num_liberties_if(&self.inner, opponent, index, &mut liberties),
                        8
                    );

                    for i in 0..num_liberties {
                        features[O::index(29+i, other)] = c_1;
                    }
                }
            }
        }

        // vertex properties
        let mut is_ko = c_0;

        for index in 0..361 {
            let other = symmetry_table[index] as usize;

            if self.inner.vertices[index].color() != 0 {
                // pass
            } else if _is_valid_memoize(&self.inner, to_move, index, &mut liberties) {
                // is super-ko
                if self._is_ko(to_move, index) {
                    is_ko = c_1;

                    features[O::index(37, other)] = c_1;
                }

                // is ladder capture
                if self.inner.is_ladder_capture(to_move, index) {
                    features[O::index(38, other)] = c_1;
                }

                // is ladder escape
                if self.inner.is_ladder_escape(to_move, index) {
                    features[O::index(39, other)] = c_1;
                }
            }
        }

        // global properties
        let c_komi = T::from(max(min(0.5 + (0.5 * self.komi) / 7.5, 1.0), 0.0));

        let is_black = if to_move == Color::Black { c_komi } else { c_0 };
        let is_white = if to_move == Color::White { c_komi } else { c_0 };

        for index in 0..361 {
            let other = symmetry_table[index] as usize;

            features[O::index(0, other)] = is_black;
            features[O::index(1, other)] = is_white;
            features[O::index(2, other)] = is_ko;
        }

        features
    }
}

/// Fills the given array with all liberties of in the provided array of vertices
/// for the group.
///
/// # Arguments
///
/// * `vertices` - the array to fill liberties from
/// * `index` - the group to fill liberties for
/// * `liberties` - output array containing the liberties of this group
///
fn fill_liberties(board: &BoardFast, index: usize, liberties: &mut [u8]) {
    for current in board.block_at(index) {
        for (other_index, vertex) in board.adjacent_to(current) {
            liberties[other_index] = vertex.color();
        }
    }
}

/// Returns the number of liberties of the given group using any recorded
/// value in `memoize` if available otherwise it is calculated. Any
/// calculated value is written back to `memoize` for all strongly
/// connected stones.
///
/// # Arguments
///
/// * `board` - 
/// * `index` - the index of the group to check
/// * `memoize` - cache of already calculated liberty counts
///
fn get_num_liberties(board: &BoardFast, index: usize, memoize: &mut [usize]) -> usize {
    if memoize[index] != 0 {
        memoize[index]
    } else {
        let mut liberties = [0xff; 384];
        fill_liberties(board, index, &mut liberties);

        // update the cached value in the memoize array for all stones
        // that are strongly connected to the given index
        let num_liberties = count_zeros(&liberties);

        for current in board.block_at(index) {
            memoize[current] = num_liberties;
        }

        num_liberties
    }
}

/// Returns whether the given move is valid according to the
/// Tromp-Taylor rules using the provided `memoize` table to
/// determine the number of liberties.
///
/// This function also assume the given vertex is empty and does
/// not perform the check itself.
///
/// # Arguments
///
/// * `color` - the color of the move
/// * `index` - the HW index of the move
/// * `memoize` - cache of already calculated liberty counts
///
fn _is_valid_memoize(board: &BoardFast, color: Color, index: usize, memoize: &mut [usize]) -> bool {
    debug_assert!(board.vertices[index].color() == 0);

    let current = color as u8;

    board.adjacent_to(index).any(|(other_index, vertex)| {
        let value = vertex.color();

        // check for direct liberties
        if value == 0 {
            return true;
        }

        // check for the following two conditions simplified into one case:
        //
        // 1. If a neighbour is friendly then we are fine if it has at
        //    least two liberties.
        // 2. If a neighbour is unfriendly then we are fine if it has less
        //    than two liberties (i.e. one).
        value != 0x3 && (value == current) == (get_num_liberties(board, other_index, memoize) >= 2)
    })
}

/// Returns the number of liberties of the group connected to the given stone
/// *if* it was played, will panic if the vertex is not empty.
///
/// # Arguments
///
/// * `color` - the color of the stone to pretend place
/// * `index` - the index of the stone to pretend place
///
fn get_num_liberties_if(board: &BoardFast, color: Color, index: usize, memoize: &mut [usize]) -> usize {
    debug_assert!(board.vertices[index].color() == 0);

    let mut other = board.clone();
    other.vertices[index].set_color(color as u8);
    other.vertices[index].set_visited(true);

    // capture of opponent stones
    let current = color as u8;
    let opponent = color.opposite() as u8;

    for (other_index, other_vertex) in board.adjacent_to(index) {
        if other_vertex.color() == opponent && get_num_liberties(&board, other_index, memoize) == 1 {
            other.capture(opponent as usize, other_index);
        }
    }

    // add liberties based on the liberties of the friendly neighbouring
    // groups
    let mut liberties = [0xff; 384];

    for (other_index, other_vertex) in other.adjacent_to(index) {
        let other_color = other_vertex.color();

        if other_color == current {
            fill_liberties(&other, other_index, &mut liberties);
        }

        // add direct liberties of the new stone
        liberties[other_index] = other_color;
    }

    count_zeros(&liberties)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_features_chw() {
        let features = Board::new(0.5)
            .get_features::<CHW, f32>(Color::Black, symmetry::Transform::Identity);

        assert_eq!(features.len(), FEATURE_SIZE);
    }

    #[test]
    fn check_features_hwc() {
        let features = Board::new(0.5)
            .get_features::<HWC, f32>(Color::Black, symmetry::Transform::Identity);

        assert_eq!(features.len(), FEATURE_SIZE);
    }
}
