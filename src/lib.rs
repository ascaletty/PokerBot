use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod poker_server {

    use pyo3::prelude::*;
    const RANK_ORDER: &str = "23456789TJQKA";
    const SUIT_ORDER: &str = "hcds"; // Hearts, Diamonds, Clubs, Spades

    #[derive(Copy, Clone)]
    struct Card {
        rank: u8, // 0..12
        suit: u8, // 0..3
    }
    fn build_masks(cards: &[Card]) -> (u16, [u16; 4], [u8; 13]) {
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
    fn evaluate(cards: &[Card]) -> (u8, Vec<u8>) {
        let (rank_mask, suit_masks, rank_count) = build_masks(cards);

        let flush_suit = flush_suit(&suit_masks);
        let straight_high = is_straight(rank_mask);

        // ---- build rank groups ----
        let mut groups: Vec<(u8, u8)> = (0..13)
            .filter(|&r| rank_count[r] > 0)
            .map(|r| (r as u8, rank_count[r]))
            .collect();

        groups.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

        let mut flush_mask: Option<u16> = None;
        if let Some(s) = flush_suit {
            flush_mask = Some(suit_masks[s]);
        }

        // ---- STRAIGHT FLUSH ----
        if let Some(fm) = flush_mask {
            if let Some(high) = is_straight(fm) {
                println!("straight flush");
                return (8, vec![high]);
            }
        }

        // ---- FOUR OF A KIND ----
        if groups[0].1 == 4 {
            println!("four of a kind");
            return (7, vec![groups[0].0, groups[1].0]);
        }

        // ---- FULL HOUSE ----
        if groups[0].1 == 3 && groups[1].1 == 2 {
            println!("full house");
            return (6, vec![groups[0].0, groups[1].0]);
        }

        // ---- FLUSH ----
        if let Some(fm) = flush_mask {
            let mut ranks: Vec<u8> = (0..13)
                .filter(|&r| (fm & (1 << r)) != 0)
                .map(|r| r as u8)
                .collect();

            ranks.sort_by(|a, b| b.cmp(a));

            println!("flush");
            return (5, ranks);
        }

        // ---- STRAIGHT ----
        if let Some(high) = straight_high {
            println!("straight_high");
            return (4, vec![high]);
        }

        // ---- THREE OF A KIND ----
        if groups[0].1 == 3 {
            let mut kickers: Vec<u8> = groups.iter().filter(|g| g.1 == 1).map(|g| g.0).collect();

            kickers.sort_by(|a, b| b.cmp(a));

            let mut res = vec![groups[0].0];
            res.extend(kickers);

            println!("three pair");
            return (3, res);
        }

        // ---- TWO PAIR ----
        if groups[0].1 == 2 && groups[1].1 == 2 {
            let mut pairs = vec![groups[0].0, groups[1].0];
            pairs.sort_by(|a, b| b.cmp(a));

            let kicker = groups.iter().find(|g| g.1 == 1).unwrap().0;

            pairs.push(kicker);

            println!("two pair");
            return (2, pairs);
        }

        // ---- PAIR ----
        if groups[0].1 == 2 {
            let mut kickers: Vec<u8> = groups.iter().filter(|g| g.1 == 1).map(|g| g.0).collect();

            kickers.sort_by(|a, b| b.cmp(a));

            let mut res = vec![groups[0].0];
            res.extend(kickers);

            println!("pair");
            return (1, res);
        }

        // ---- HIGH CARD ----
        let mut ranks: Vec<u8> = (0..13)
            .filter(|&r| rank_count[r] > 0)
            .map(|r| r as u8)
            .collect();
        println!("high card");

        ranks.sort_by(|a, b| b.cmp(a));
        (0, ranks)
    }
}
#[cfg(test)]
mod test {
    #[derive(Copy, Clone)]
    struct Card {
        rank: u8, // 0..12
        suit: u8, // 0..3
    }
    fn build_masks(cards: &[Card]) -> (u16, [u16; 4], [u8; 13]) {
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
    fn evaluate(cards: &[Card]) -> (u8, Vec<u8>) {
        let (rank_mask, suit_masks, rank_count) = build_masks(cards);

        let flush_suit = flush_suit(&suit_masks);
        let straight_high = is_straight(rank_mask);

        // ---- build rank groups ----
        let mut groups: Vec<(u8, u8)> = (0..13)
            .filter(|&r| rank_count[r] > 0)
            .map(|r| (r as u8, rank_count[r]))
            .collect();

        groups.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

        let mut flush_mask: Option<u16> = None;
        if let Some(s) = flush_suit {
            flush_mask = Some(suit_masks[s]);
        }

        // ---- STRAIGHT FLUSH ----
        if let Some(fm) = flush_mask {
            if let Some(high) = is_straight(fm) {
                println!("straight flush");
                return (8, vec![high]);
            }
        }

        // ---- FOUR OF A KIND ----
        if groups[0].1 == 4 {
            println!("four of a kind");
            return (7, vec![groups[0].0, groups[1].0]);
        }

        // ---- FULL HOUSE ----
        if groups[0].1 == 3 && groups[1].1 == 2 {
            println!("full house");
            return (6, vec![groups[0].0, groups[1].0]);
        }

        // ---- FLUSH ----
        if let Some(fm) = flush_mask {
            let mut ranks: Vec<u8> = (0..13)
                .filter(|&r| (fm & (1 << r)) != 0)
                .map(|r| r as u8)
                .collect();

            ranks.sort_by(|a, b| b.cmp(a));

            println!("flush");
            return (5, ranks);
        }

        // ---- STRAIGHT ----
        if let Some(high) = straight_high {
            println!("straight_high");
            return (4, vec![high]);
        }

        // ---- THREE OF A KIND ----
        if groups[0].1 == 3 {
            let mut kickers: Vec<u8> = groups.iter().filter(|g| g.1 == 1).map(|g| g.0).collect();

            kickers.sort_by(|a, b| b.cmp(a));

            let mut res = vec![groups[0].0];
            res.extend(kickers);

            println!("three pair");
            return (3, res);
        }

        // ---- TWO PAIR ----
        if groups[0].1 == 2 && groups[1].1 == 2 {
            let mut pairs = vec![groups[0].0, groups[1].0];
            pairs.sort_by(|a, b| b.cmp(a));

            let kicker = groups.iter().find(|g| g.1 == 1).unwrap().0;

            pairs.push(kicker);

            println!("two pair");
            return (2, pairs);
        }

        // ---- PAIR ----
        if groups[0].1 == 2 {
            let mut kickers: Vec<u8> = groups.iter().filter(|g| g.1 == 1).map(|g| g.0).collect();

            kickers.sort_by(|a, b| b.cmp(a));

            let mut res = vec![groups[0].0];
            res.extend(kickers);

            println!("pair");
            return (1, res);
        }

        // ---- HIGH CARD ----
        let mut ranks: Vec<u8> = (0..13)
            .filter(|&r| rank_count[r] > 0)
            .map(|r| r as u8)
            .collect();
        println!("high card");

        ranks.sort_by(|a, b| b.cmp(a));
        (0, ranks)
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
