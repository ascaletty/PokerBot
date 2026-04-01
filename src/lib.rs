use core::fmt;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
use pyo3::prelude::*;
#[pyfunction]
fn parse_card(s: &str) -> u8 {
    let rank = match s.as_bytes()[0] {
        b'2' => 0,
        b'3' => 1,
        b'4' => 2,
        b'5' => 3,
        b'6' => 4,
        b'7' => 5,
        b'8' => 6,
        b'9' => 7,
        b'T' => 8,
        b'J' => 9,
        b'Q' => 10,
        b'K' => 11,
        b'A' => 12,
        _ => panic!("bad rank"),
    };

    let suit = match s.as_bytes()[1] {
        b'h' => 0,
        b'd' => 1,
        b'c' => 2,
        b's' => 3,
        _ => panic!("bad suit"),
    };

    suit * 13 + rank
}
type Card = u8; // 0..51
type DeckMask = u64; // 52-bit mask
fn bit(card: u8) -> u64 {
    1u64 << card
}
fn build_used_mask(my: &[Card], board: &[Card]) -> DeckMask {
    let mut used = 0u64;

    for &c in my.iter().chain(board.iter()) {
        used |= bit(c);
    }

    used
}
fn build_deck_excluding(used: DeckMask) -> ([Card; 47], usize) {
    let mut deck = [0u8; 47];
    let mut len = 0;

    for c in 0u8..52 {
        if used & (1u64 << c) == 0 {
            deck[len] = c;
            len += 1;
        }
    }

    (deck, len)
}
#[inline(always)]
fn eval_7_fast_u8(cards: &[u8; 7]) -> u32 {
    let mut rank_mask: u16 = 0;
    let mut suit_mask: [u16; 4] = [0; 4];
    let mut rank_count: [u8; 13] = [0; 13];

    // ---------------------------
    // Build bitmasks
    // ---------------------------
    for &c in cards {
        let r = (c % 13) as usize;
        let s = (c / 13) as usize;
        let bit = 1u16 << r;

        rank_mask |= bit;
        suit_mask[s] |= bit;
        rank_count[r] += 1;
    }

    // ---------------------------
    // Detect groups
    // ---------------------------
    let mut four = None;
    let mut trips = [None; 2];
    let mut pairs = [None; 3];

    let mut t_i = 0;
    let mut p_i = 0;

    for r in (0..13).rev() {
        match rank_count[r] {
            4 => four = Some(r as u8),
            3 => {
                if t_i < 2 {
                    trips[t_i] = Some(r as u8);
                    t_i += 1;
                }
            }
            2 => {
                if p_i < 3 {
                    pairs[p_i] = Some(r as u8);
                    p_i += 1;
                }
            }
            _ => {}
        }
    }

    // ---------------------------
    // helpers
    // ---------------------------
    #[inline(always)]
    fn top_kicker(mut mask: u16, k: usize) -> u32 {
        let mut res = 0;
        let mut cnt = 0;

        for r in (0..13).rev() {
            if mask & (1 << r) != 0 {
                res = res * 13 + r as u32;
                cnt += 1;
                if cnt == k {
                    break;
                }
            }
        }
        res
    }

    #[inline(always)]
    fn is_straight(mask: u16) -> Option<u8> {
        for i in (0..10).rev() {
            if mask & (0b11111 << i) == (0b11111 << i) {
                return Some((i + 4) as u8);
            }
        }
        // wheel A2345
        if mask & 0b1000000001111 == 0b1000000001111 {
            return Some(3);
        }
        None
    }

    // ---------------------------
    // Straight Flush
    // ---------------------------
    for s in 0..4 {
        if let Some(high) = is_straight(suit_mask[s]) {
            return 8 + high as u32;
        }
    }

    // ---------------------------
    // Four of a kind
    // ---------------------------
    if let Some(q) = four {
        let kicker_mask = rank_mask & !(1 << q);
        return 7 + (q as u32 * 13 + top_kicker(kicker_mask, 1));
    }

    // ---------------------------
    // Full house
    // ---------------------------
    if let Some(t) = trips[0] {
        if let Some(p) = pairs[0].or(trips[1]) {
            return 6 + (t as u32 * 13 + p as u32);
        }
    }

    // ---------------------------
    // Flush
    // ---------------------------
    for s in 0..4 {
        if suit_mask[s].count_ones() >= 5 {
            return 5_000_000 + top_kicker(suit_mask[s], 5);
        }
    }

    // ---------------------------
    // Straight
    // ---------------------------
    if let Some(high) = is_straight(rank_mask) {
        return 4_000_000 + high as u32;
    }

    // ---------------------------
    // Three of a kind
    // ---------------------------
    if let Some(t) = trips[0] {
        let m = rank_mask & !(1 << t);
        return 3_000_000 + (t as u32 * 169 + top_kicker(m, 2));
    }

    // ---------------------------
    // Two pair
    // ---------------------------
    if let (Some(p1), Some(p2)) = (pairs[0], pairs[1]) {
        let m = rank_mask & !(1 << p1) & !(1 << p2);
        return 2_000_000 + (p1 as u32 * 169 + p2 as u32 * 13 + top_kicker(m, 1));
    }

    // ---------------------------
    // One pair
    // ---------------------------
    if let Some(p) = pairs[0] {
        let m = rank_mask & !(1 << p);
        return 1_000_000 + (p as u32 * 2197 + top_kicker(m, 3));
    }

    // ---------------------------
    // High card
    // ---------------------------
    0 + top_kicker(rank_mask, 5)
}
#[inline(always)]
fn remove2(deck: &[u8], i: usize, j: usize) -> ([u8; 2], usize) {
    debug_assert!(i != j);
    debug_assert!(i < deck.len() && j < deck.len());

    let a = deck[i];
    let b = deck[j];

    ([a, b], 2)
}
#[rustfmt::skip]
const HAND_RANKING: [f32; 169] = [
    1.00, // AA (0)
    0.99, // KK (1)
    0.98, // QQ (2)
    0.97, // JJ (3)
    0.96, // TT (4)
    0.95, // 99 (5)
    0.80, // 88 (6)
    0.70, // 77 (7)
    0.65, // 66 (8)
    0.60, // 55 (9)
    0.60, // 44 (10)
    0.55, // 33 (11)
    0.40, // 22 (12)
    0.98, 0.97, 0.95, 0.94, 0.93, 0.92, 0.91, 0.90, 0.89, 0.85, 0.84,0.83, // AKs → A2s (13–24)
    0.97, 0.96, 0.95, 0.94, 0.93, 0.92, 0.91, 0.90, 0.89, 0.85, 0.84, // KQs → K2s (25–35)
    0.87, 0.85, 0.84, 0.83, 0.82, 0.81, 0.80, 0.79, 0.78,0.75, // QJs → Q2s (36–45)
    0.74, 0.73, 0.71, 0.65, 0.64, 0.63, 0.62, 0.60,// JTs → J2s (46–53)
    0.65, 0.60, 0.35, 0.25, 0.20, 0.0, 0.0,  // T9s → T2s (54–60)
    0.60, 0.35, 0.25, 0.20, 0.0, 0.00, // 98s → 92s (61–66)
    0.50, 0.40, 0.10, 0.05, 0.02, // 87s → 82s (67–71)
    0.45, 0.30, 0.10, 0.0, 0.00, // 76s → 72s (72–76)
    0.40, 0.30, 0.0, 0.00, // 65s → 62s (77–80)
    0.30, 0.0, 0.00, // 54s → 52s (81–83)
    0.24, 0.0, // 43s → 42s (84–85)
    0.20, // 32s (86)
    0.98, //AKo (87)
    0.90, 0.87, 0.86, 0.80, 0.78, 0.75, 0.75, 0.73, 0.72, 0.70, 0.72,0.71, // AQo → A2o (88–100)
    0.65, 0.60, 0.55, 0.50, 0.45, 0.40, 0.35, 0.30, 0.20, 0.10, 0.0, // KQo → K2o (101–113)
    0.65, 0.45, 0.0, 0.0, 0.4, 0.3, 0.2, 0.10, 0.0, 0.0,0.0, // QJo → Q2o (114–124)
    0.50, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,0.0,0.0,  // JTo → J2o (125–135)
    0.60, 0.35, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,  // T9o → T2o (136–143)
    0.40, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //98o->92o
    0.30, 0.0, 0.0, 0.0, 0.0, 0.0, //87o-> 82o
    0.20, 0.0, 0.0, 0.0, 0.0, //76o->72o
    0.0, 0.0, 0.0, 0.0, //65o->62o
    0.0, 0.0, 0.0, //54o->52o
    0.0, 0.0, //43o-> 42o
    0.0, //32o
];
const Hand_Ranking: [f64; 169] = [
    // AA thru A2s
    1.0, 0.98, 0.97, 0.96, 0.95, 0.94, 0.93, 0.92, 0.91, 0.90, 0.89, 0.89, 0.88,
    //AKo thru K2s
    0.90, 0.99, 0.97, 0.96, 0.95, 0.94, 0.93, 0.92, 0.91, 0.90, 0.89, 0.85, 0.84,
    //AQo thru Q2s
    0.87, 0.87, 0.98, 0.87, 0.85, 0.84, 0.83, 0.82, 0.81, 0.80, 0.79, 0.78, 0.77,
    //AJo thru J2s
    0.80, 0.78, 0.75, 0.96, 0.74, 0.73, 0.72, 0.71, 0.65, 0.65, 0.63, 0.62, 0.60,
    //ATo T2s
    0.75, 0.73, 0.72, 0.70, 0.95, 0.65, 0.60, 0.35, 0.25, 0.20, 0.0, 0.0, //A9o thru 92s
    0.72, 0.65, 0.60, 0.55, 0.50, 0.80, 0.60, 0.50, 0.40, 0.10, 0.05, 0.02,
    //A8o thru 82s
    0.70, 0.55, 0.50, 0.45, 0.40, 0.35, 0.70, 0.55, 0.50, 0.30, 0.20, 0.10,
    //A7o thru 72s
    0.65, 0.45, 0.0, 0.0, 0.40, 0.30, 0.20, 0.65, 0.60, 0.50, 0.30, 0.0, 0.0,
    //A6o thru 62s
    0.60, 0.35, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.60, 0.45, 0.30, 0.0, 0.0, //A5o thru 52s
    0.5, 0.35, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.60, 0.30, 0.25, 0.0, //A4o thru 42s
    0.40, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.55, 0.25, //A3o thru 32s
    0.30, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.50, 0.0,
    //A2o thru 2s
    0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.40,
];

#[pyfunction]
pub fn hand_to_index(a: u8, b: u8) -> usize {
    let ra = (a % 13) as usize;
    let rb = (b % 13) as usize;

    let sa = a / 13;
    let sb = b / 13;

    let (hi, lo, suited) = if ra > rb {
        (ra, rb, sa == sb)
    } else {
        (rb, ra, sa == sb)
    };

    // -------------------------
    // 1) Pocket pairs: 0..12
    // AA(12)->0, KK(11)->1, ..., 22(0)->12
    // -------------------------
    if hi == lo {
        return 12 - hi;
    }

    // -------------------------
    // 2) Suited hands: 13..90
    // lexicographic ordering: AKs highest, 32s lowest
    // -------------------------
    let mut idx = 0;
    if suited {
        for r1 in (0..=12).rev() {
            for r2 in (0..r1).rev() {
                if r1 == hi && r2 == lo {
                    return 13 + idx;
                }
                idx += 1;
            }
        }
    }

    // -------------------------
    // 3) Offsuit hands: 91..168
    // same ordering as suited block
    // -------------------------
    let mut idx = 0;

    for r1 in (0..=12).rev() {
        for r2 in (0..r1).rev() {
            if r1 == hi && r2 == lo {
                return 13 + 78 + idx;
            }
            idx += 1;
        }
    }

    unreachable!()
}
#[pyfunction]
fn hand_to_ranking(cards: Vec<String>) -> (f32, usize) {
    let mut cards_parsed = [0; 2];
    cards_parsed[0] = parse_card(&cards[0]);

    cards_parsed[1] = parse_card(&cards[1]);

    let index = hand_to_index(cards_parsed[0], cards_parsed[1]);
    print!("index{}", index);
    (HAND_RANKING[index], index)
}
#[pyfunction]
fn exact_flop_equity(my: [u8; 2], flop: [u8; 3]) -> (u64, u64) {
    let (mut deck, deck_len) = build_deck_excluding(build_used_mask(&my, &flop));

    let mut beat = 0u64;
    let mut total = 0u64;

    let mut board = [0u8; 7];

    for i in 0..deck_len {
        for j in i + 1..deck_len {
            let opp = [deck[i], deck[j]];

            // remaining cards for turn/river
            let mut deck2 = deck;

            remove2(&deck2, i, j);
            let len2 = deck2.len();

            for t in 0..len2 {
                for r in t + 1..len2 {
                    if t == i || t == j || r == i || r == j {
                    } else {
                        board[0] = my[0];
                        board[1] = my[1];
                        board[2] = flop[0];
                        board[3] = flop[1];
                        board[4] = flop[2];
                        board[5] = deck2[t];
                        board[6] = deck2[r];

                        let my_s = eval_7_fast_u8(&board);

                        board[0] = opp[0];
                        board[1] = opp[1];

                        let opp_s = eval_7_fast_u8(&board);

                        total += 1;
                        if opp_s > my_s {
                            beat += 1;
                        }
                    }
                }
            }
        }
    }

    print!("equity is {}", beat / total);
    (beat, total)
}
#[pymodule]
fn poker_server_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(exact_flop_equity, m)?)?;
    m.add_function(wrap_pyfunction!(parse_card, m)?)?;
    m.add_function(wrap_pyfunction!(hand_to_index, m)?)?;
    m.add_function(wrap_pyfunction!(hand_to_ranking, m)?)?;
    Ok(())
}
