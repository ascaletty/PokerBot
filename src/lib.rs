use pyo3::prelude::*;

/// A Python module implemented in Rust.
use pyo3::prelude::*;
const RANK_ORDER: &str = "23456789TJQKA";
const SUIT_ORDER: &str = "hcds"; // Hearts, Diamonds, Clubs, Spades

#[derive(Copy, Clone)]
struct Card {
    rank: u8, // 0..12
    suit: u8, // 0..3
}
fn parse_cards(input: Vec<String>) -> Result<Vec<Card>, String> {
    fn rank_to_u8(c: char) -> Option<u8> {
        match c {
            '2' => Some(0),
            '3' => Some(1),
            '4' => Some(2),
            '5' => Some(3),
            '6' => Some(4),
            '7' => Some(5),
            '8' => Some(6),
            '9' => Some(7),
            'T' => Some(8),
            'J' => Some(9),
            'Q' => Some(10),
            'K' => Some(11),
            'A' => Some(12),
            _ => None,
        }
    }

    fn suit_to_u8(c: char) -> Option<u8> {
        match c {
            'h' => Some(0),
            'd' => Some(1),
            'c' => Some(2),
            's' => Some(3),
            _ => None,
        }
    }

    input
        .into_iter()
        .map(|s| {
            if s.len() != 2 {
                return Err(format!("Invalid card format: {}", s));
            }

            let mut chars = s.chars();
            let rank_char = chars.next().unwrap();
            let suit_char = chars.next().unwrap();

            let rank =
                rank_to_u8(rank_char).ok_or_else(|| format!("Invalid rank: {}", rank_char))?;

            let suit =
                suit_to_u8(suit_char).ok_or_else(|| format!("Invalid suit: {}", suit_char))?;

            Ok(Card { rank, suit })
        })
        .collect()
}
fn build_masks(cards: Vec<Card>) -> (u16, [u16; 4], [u8; 13]) {
    let mut rank_mask: u16 = 0;
    let mut suit_masks = [0u16; 4];
    let mut rank_count = [0u8; 13];

    for c in cards {
        let bit = 1 << c.rank;

        rank_mask |= bit;
        suit_masks[c.suit as usize] |= bit;
        rank_count[c.rank as usize] += 1;
    }

    (rank_mask, suit_masks, rank_count)
}
fn is_straight(mask: u16) -> Option<u8> {
    // normal straights
    for i in 0..9 {
        if (mask >> i) & 0b11111 == 0b11111 {
            return Some(i + 4); // high card of straight
        }
    }

    // wheel: A-2-3-4-5
    if (mask & 0b1000000001111) == 0b1000000001111 {
        return Some(3);
    }

    None
}

fn flush_suit(suits: &[u16; 4]) -> Option<usize> {
    suits.iter().position(|&m| m.count_ones() >= 5)
}
fn card_to_bit(card: &str) -> (u16, usize) {
    let bytes = card.as_bytes();

    let rank_char = bytes[0] as char;
    let suit_char = bytes[1] as char;

    let rank = RANK_ORDER.find(rank_char).unwrap() as u16;
    let suit = SUIT_ORDER.find(suit_char).unwrap();

    let bit = 1 << rank;

    (bit, suit)
}
fn hand_to_bitmasks(cards: &[&str]) -> (u16, [u16; 4]) {
    let mut rank_mask: u16 = 0;
    let mut suit_masks = [0u16; 4];

    for &card in cards {
        let (bit, suit) = card_to_bit(card);
        rank_mask |= bit;
        suit_masks[suit] |= bit;
    }

    (rank_mask, suit_masks)
}
#[pyfunction]

fn evaluate(cards: Vec<String>) -> u32 {
    let cards = parse_cards(cards).unwrap();
    let (rank_mask, suit_masks, rank_count) = build_masks(cards);

    let flush_suit_idx = flush_suit(&suit_masks);
    let straight_high = is_straight(rank_mask);

    // ---- build rank groups ----
    let mut groups: Vec<(u8, u8)> = (0..13)
        .filter(|&r| rank_count[r] > 0)
        .map(|r| (r as u8, rank_count[r]))
        .collect();

    groups.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    let flush_mask = flush_suit_idx.map(|s| suit_masks[s]);

    // helper: pack ranks into a number (base-13 encoding)
    fn encode_ranks(ranks: &[u8]) -> u32 {
        ranks.iter().fold(0, |acc, &r| acc * 13 + r as u32)
    }

    const BASE_SF: u32 = 0;
    const BASE_4K: u32 = 10;
    const BASE_FH: u32 = BASE_4K + 156;
    const BASE_FL: u32 = BASE_FH + 156;
    const BASE_ST: u32 = BASE_FL + 1277;
    const BASE_3K: u32 = BASE_ST + 10;
    const BASE_2P: u32 = BASE_3K + 858;
    const BASE_1P: u32 = BASE_2P + 858;
    const BASE_HC: u32 = BASE_1P + 2860;

    // ---- STRAIGHT FLUSH ----
    if let Some(fm) = flush_mask {
        if let Some(high) = is_straight(fm) {
            return BASE_SF + (12 - high as u32);
        }
    }

    // ---- FOUR OF A KIND ----
    if groups[0].1 == 4 {
        let quad = groups[0].0;
        let kicker = groups[1].0;
        return BASE_4K + ((12 - quad as u32) * 12 + (12 - kicker as u32));
    }

    // ---- FULL HOUSE ----
    if groups[0].1 == 3 && groups[1].1 >= 2 {
        let trip = groups[0].0;
        let pair = groups[1].0;
        return BASE_FH + ((12 - trip as u32) * 12 + (12 - pair as u32));
    }

    // ---- FLUSH ----
    if let Some(fm) = flush_mask {
        let mut ranks: Vec<u8> = (0..13)
            .filter(|&r| (fm & (1 << r)) != 0)
            .map(|r| r as u8)
            .collect();

        ranks.sort_by(|a, b| b.cmp(a));

        return BASE_FL + encode_ranks(&ranks);
    }

    // ---- STRAIGHT ----
    if let Some(high) = straight_high {
        return BASE_ST + (12 - high as u32);
    }

    // ---- THREE OF A KIND ----
    if groups[0].1 == 3 {
        let mut kickers: Vec<u8> = groups.iter().filter(|g| g.1 == 1).map(|g| g.0).collect();

        kickers.sort_by(|a, b| b.cmp(a));

        let mut ranks = vec![groups[0].0];
        ranks.extend(kickers);

        return BASE_3K + encode_ranks(&ranks);
    }

    // ---- TWO PAIR ----
    if groups[0].1 == 2 && groups[1].1 == 2 {
        let mut pairs = vec![groups[0].0, groups[1].0];
        pairs.sort_by(|a, b| b.cmp(a));

        let kicker = groups.iter().find(|g| g.1 == 1).unwrap().0;

        let mut ranks = pairs;
        ranks.push(kicker);

        return BASE_2P + encode_ranks(&ranks);
    }

    // ---- ONE PAIR ----
    if groups[0].1 == 2 {
        let mut kickers: Vec<u8> = groups.iter().filter(|g| g.1 == 1).map(|g| g.0).collect();

        kickers.sort_by(|a, b| b.cmp(a));

        let mut ranks = vec![groups[0].0];
        ranks.extend(kickers);

        return BASE_1P + encode_ranks(&ranks);
    }

    // ---- HIGH CARD ----
    let mut ranks: Vec<u8> = (0..13)
        .filter(|&r| rank_count[r] > 0)
        .map(|r| r as u8)
        .collect();

    ranks.sort_by(|a, b| b.cmp(a));

    BASE_HC + encode_ranks(&ranks)
}
#[pymodule]
fn poker_server_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;

    Ok(())
}
#[cfg(test)]
mod test {
    #[derive(Copy, Clone)]
    struct Card {
        rank: u8, // 0..12
        suit: u8, // 0..3
    }
    #[test]
    fn test1() {
        let cards = [
            Card { rank: 12, suit: 1 }, // A
            Card { rank: 12, suit: 2 }, // A
            Card { rank: 10, suit: 1 }, // Q
            Card { rank: 9, suit: 1 },  // J
            Card { rank: 8, suit: 1 },  // T
            Card { rank: 2, suit: 1 },
        ];

        let result = evaluate(&cards);

        println!("{:?}", result);
    }
}
