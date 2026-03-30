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
            return 8_000_000 + high as u32;
        }
    }

    // ---------------------------
    // Four of a kind
    // ---------------------------
    if let Some(q) = four {
        let kicker_mask = rank_mask & !(1 << q);
        return 7_000_000 + (q as u32 * 13 + top_kicker(kicker_mask, 1));
    }

    // ---------------------------
    // Full house
    // ---------------------------
    if let Some(t) = trips[0] {
        if let Some(p) = pairs[0].or(trips[1]) {
            return 6_000_000 + (t as u32 * 13 + p as u32);
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

    Ok(())
}
