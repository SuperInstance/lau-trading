use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Items that can be traded between players: materials, agents, knowledge, recipes, and pets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Tradeable {
    Material(String, usize),
    Agent(String, f64),
    Knowledge(String, f64),
    Recipe(String),
    Pet(String, u32),
}

/// Status of a trade offer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeStatus {
    Pending,
    Accepted,
    Rejected,
    Expired,
}

/// An offer to trade items between two players.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradeOffer {
    pub id: String,
    pub from: String,
    pub to: String,
    pub give: Vec<Tradeable>,
    pub want: Vec<Tradeable>,
    pub status: TradeStatus,
}

impl TradeOffer {
    /// Simple fairness check: both sides have the same number of items.
    pub fn is_fair(&self) -> bool {
        self.give.len() == self.want.len()
    }
}

/// A completed trade recorded in history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletedTrade {
    pub offer: TradeOffer,
    pub completed_tick: u64,
}

/// Trading statistics derived from completed trade history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradeStats {
    pub total_trades: u64,
    pub by_type: HashMap<String, u64>,
    pub most_active: Option<String>,
}

impl TradeStats {
    /// Build statistics from a slice of completed trades.
    pub fn from_history(history: &[CompletedTrade]) -> Self {
        let total_trades = history.len() as u64;
        let mut by_type: HashMap<String, u64> = HashMap::new();
        let mut player_counts: HashMap<String, u64> = HashMap::new();

        for ct in history {
            for item in &ct.offer.give {
                let key = match item {
                    Tradeable::Material(_, _) => "material",
                    Tradeable::Agent(_, _) => "agent",
                    Tradeable::Knowledge(_, _) => "knowledge",
                    Tradeable::Recipe(_) => "recipe",
                    Tradeable::Pet(_, _) => "pet",
                };
                *by_type.entry(key.to_string()).or_insert(0) += 1;
            }
            *player_counts.entry(ct.offer.from.clone()).or_insert(0) += 1;
            *player_counts.entry(ct.offer.to.clone()).or_insert(0) += 1;
        }

        let most_active = player_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(player, _)| player);

        TradeStats {
            total_trades,
            by_type,
            most_active,
        }
    }
}

/// A one-way gift transfer (no negotiation needed).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gift {
    pub from: String,
    pub to: String,
    pub item: Tradeable,
    pub message: String,
}

/// The central marketplace for creating, finding, and settling trades.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradeMarket {
    pub offers: Vec<TradeOffer>,
    pub history: Vec<CompletedTrade>,
}

impl TradeMarket {
    pub fn new() -> Self {
        TradeMarket {
            offers: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Create a new trade offer and return its id.
    pub fn create_offer(
        &mut self,
        from: &str,
        to: &str,
        give: Vec<Tradeable>,
        want: Vec<Tradeable>,
    ) -> String {
        let id = format!("offer-{}", self.offers.len() + self.history.len() + 1);

        let offer = TradeOffer {
            id: id.clone(),
            from: from.to_string(),
            to: to.to_string(),
            give,
            want,
            status: TradeStatus::Pending,
        };

        self.offers.push(offer);
        id
    }

    /// Accept a pending offer, moving it to history and returning the completed trade.
    pub fn accept(&mut self, offer_id: &str) -> Result<CompletedTrade, String> {
        let pos = self
            .offers
            .iter()
            .position(|o| o.id == offer_id)
            .ok_or_else(|| format!("Offer {} not found", offer_id))?;

        let mut offer = self.offers.remove(pos);

        if offer.status != TradeStatus::Pending {
            return Err(format!("Offer {} is not pending (status: {:?})", offer_id, offer.status));
        }

        offer.status = TradeStatus::Accepted;

        let completed_tick = (self.history.len() + self.offers.len() + 1) as u64;
        let completed = CompletedTrade { offer, completed_tick };

        self.history.push(completed.clone());
        Ok(completed)
    }

    /// Reject a pending offer.
    pub fn reject(&mut self, offer_id: &str) -> Result<(), String> {
        let offer = self
            .offers
            .iter_mut()
            .find(|o| o.id == offer_id)
            .ok_or_else(|| format!("Offer {} not found", offer_id))?;

        if offer.status != TradeStatus::Pending {
            return Err(format!("Offer {} is not pending (status: {:?})", offer_id, offer.status));
        }

        offer.status = TradeStatus::Rejected;
        Ok(())
    }

    /// Find all pending offers addressed to a specific player.
    pub fn find_offers_for(&self, player: &str) -> Vec<&TradeOffer> {
        self.offers
            .iter()
            .filter(|o| o.to == player && o.status == TradeStatus::Pending)
            .collect()
    }

    /// Find all pending offers that want a specific tradeable (who wants what I have?).
    pub fn find_offers_wanting(&self, tradeable: &Tradeable) -> Vec<&TradeOffer> {
        self.offers
            .iter()
            .filter(|o| o.status == TradeStatus::Pending && o.want.iter().any(|w| w == tradeable))
            .collect()
    }

    /// Send a one-way gift (no trade negotiation needed).
    pub fn send_gift(&mut self, gift: Gift) {
        self.history.push(CompletedTrade {
            offer: TradeOffer {
                id: format!("gift-{}", self.history.len() + self.offers.len() + 1),
                from: gift.from,
                to: gift.to,
                give: vec![gift.item],
                want: vec![],
                status: TradeStatus::Accepted,
            },
            completed_tick: (self.history.len() + self.offers.len() + 1) as u64,
        });
    }
}

impl Default for TradeMarket {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_trade_offer() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("iron".to_string(), 5)],
            vec![Tradeable::Agent("builder_bot".to_string(), 0.95)],
        );
        assert!(id.starts_with("offer-"));
        assert_eq!(market.offers.len(), 1);
    }

    #[test]
    fn test_is_fair_equal() {
        let offer = TradeOffer {
            id: "test".into(),
            from: "alice".into(),
            to: "bob".into(),
            give: vec![
                Tradeable::Material("iron".to_string(), 5),
                Tradeable::Pet("dog".to_string(), 3),
            ],
            want: vec![
                Tradeable::Agent("bot".to_string(), 0.9),
                Tradeable::Knowledge("mining".to_string(), 0.8),
            ],
            status: TradeStatus::Pending,
        };
        assert!(offer.is_fair());
    }

    #[test]
    fn test_is_fair_unequal() {
        let offer = TradeOffer {
            id: "test".into(),
            from: "alice".into(),
            to: "bob".into(),
            give: vec![Tradeable::Recipe("pizza".to_string())],
            want: vec![
                Tradeable::Agent("bot".to_string(), 0.9),
                Tradeable::Knowledge("cooking".to_string(), 0.7),
            ],
            status: TradeStatus::Pending,
        };
        assert!(!offer.is_fair());
    }

    #[test]
    fn test_accept_offer() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("wood".to_string(), 10)],
            vec![Tradeable::Pet("cat".to_string(), 5)],
        );
        let completed = market.accept(&id).unwrap();
        assert_eq!(completed.offer.status, TradeStatus::Accepted);
        assert!(completed.completed_tick > 0);
        assert!(market.offers.is_empty());
        assert_eq!(market.history.len(), 1);
    }

    #[test]
    fn test_accept_nonexistent_offer() {
        let mut market = TradeMarket::new();
        let result = market.accept("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_offer() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("stone".to_string(), 3)],
            vec![Tradeable::Recipe("bread".to_string())],
        );
        market.reject(&id).unwrap();
        assert_eq!(market.offers[0].status, TradeStatus::Rejected);
    }

    #[test]
    fn test_reject_twice_fails() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("stone".to_string(), 3)],
            vec![Tradeable::Recipe("bread".to_string())],
        );
        market.reject(&id).unwrap();
        let result = market.reject(&id);
        assert!(result.is_err());
        assert_eq!(market.offers[0].status, TradeStatus::Rejected);
    }

    #[test]
    fn test_accept_rejected_fails() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("stone".to_string(), 3)],
            vec![Tradeable::Recipe("bread".to_string())],
        );
        market.reject(&id).unwrap();
        let result = market.accept(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_offers_for() {
        let mut market = TradeMarket::new();
        market.create_offer("alice", "bob", vec![Tradeable::Material("iron".into(), 1)], vec![]);
        market.create_offer("carol", "bob", vec![Tradeable::Pet("dog".into(), 2)], vec![]);
        market.create_offer("bob", "alice", vec![Tradeable::Recipe("cake".into())], vec![]);

        let bob_offers = market.find_offers_for("bob");
        assert_eq!(bob_offers.len(), 2);
        assert_eq!(bob_offers[0].from, "alice");
        assert_eq!(bob_offers[1].from, "carol");
    }

    #[test]
    fn test_find_offers_for_ignores_non_pending() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("iron".into(), 1)],
            vec![],
        );
        market.reject(&id).unwrap();

        let bob_offers = market.find_offers_for("bob");
        assert!(bob_offers.is_empty());
    }

    #[test]
    fn test_find_offers_wanting() {
        let mut market = TradeMarket::new();
        let iron = Tradeable::Material("iron".to_string(), 5);
        market.create_offer("alice", "bob", vec![Tradeable::Recipe("sword".into())], vec![iron.clone()]);
        market.create_offer("bob", "carol", vec![Tradeable::Agent("bot".into(), 0.5)], vec![iron.clone()]);
        market.create_offer("dave", "eve", vec![Tradeable::Pet("cat".into(), 1)], vec![Tradeable::Material("wood".into(), 3)]);

        let results = market.find_offers_wanting(&iron);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_find_offers_wanting_exact_match() {
        let mut market = TradeMarket::new();
        let specific_iron = Tradeable::Material("iron".to_string(), 5);
        let different_iron = Tradeable::Material("iron".to_string(), 10);

        market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Recipe("sword".into())],
            vec![specific_iron.clone()],
        );

        let results = market.find_offers_wanting(&different_iron);
        assert!(results.is_empty(), "Should not match: quantities differ");
    }

    #[test]
    fn test_send_gift() {
        let mut market = TradeMarket::new();
        let gift = Gift {
            from: "alice".to_string(),
            to: "bob".to_string(),
            item: Tradeable::Pet("dragon".to_string(), 10),
            message: "Happy birthday!".to_string(),
        };
        market.send_gift(gift);
        assert_eq!(market.history.len(), 1);
        let ct = &market.history[0];
        assert_eq!(ct.offer.from, "alice");
        assert_eq!(ct.offer.to, "bob");
        assert_eq!(ct.offer.want.len(), 0);
    }

    #[test]
    fn test_trade_stats_empty() {
        let stats = TradeStats::from_history(&[]);
        assert_eq!(stats.total_trades, 0);
        assert!(stats.by_type.is_empty());
        assert!(stats.most_active.is_none());
    }

    #[test]
    fn test_trade_stats_with_trades() {
        let mut market = TradeMarket::new();
        market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("iron".to_string(), 5)],
            vec![Tradeable::Pet("dog".to_string(), 2)],
        );
        market.accept("offer-1").unwrap();

        let stats = TradeStats::from_history(&market.history);
        assert_eq!(stats.total_trades, 1);
        assert_eq!(*stats.by_type.get("material").unwrap(), 1);
        assert!(stats.most_active.is_some());
    }

    #[test]
    fn test_trade_stats_by_type() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![
                Tradeable::Material("iron".to_string(), 5),
                Tradeable::Agent("bot".to_string(), 0.9),
                Tradeable::Knowledge("smithing".to_string(), 0.7),
                Tradeable::Recipe("sword".to_string()),
                Tradeable::Pet("wolf".to_string(), 3),
            ],
            vec![Tradeable::Material("gold".to_string(), 1)],
        );
        market.accept(&id).unwrap();

        let stats = TradeStats::from_history(&market.history);
        assert_eq!(stats.total_trades, 1);
        assert_eq!(*stats.by_type.get("material").unwrap(), 1);
        assert_eq!(*stats.by_type.get("agent").unwrap(), 1);
        assert_eq!(*stats.by_type.get("knowledge").unwrap(), 1);
        assert_eq!(*stats.by_type.get("recipe").unwrap(), 1);
        assert_eq!(*stats.by_type.get("pet").unwrap(), 1);
    }

    #[test]
    fn test_trade_stats_most_active() {
        let mut market = TradeMarket::new();
        let id1 = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Material("wood".to_string(), 3)],
            vec![Tradeable::Material("stone".to_string(), 3)],
        );
        market.accept(&id1).unwrap();

        let id2 = market.create_offer(
            "alice",
            "carol",
            vec![Tradeable::Material("iron".to_string(), 2)],
            vec![Tradeable::Pet("cat".to_string(), 1)],
        );
        market.accept(&id2).unwrap();

        let stats = TradeStats::from_history(&market.history);
        assert_eq!(stats.total_trades, 2);
        assert_eq!(stats.most_active.as_deref(), Some("alice"));
    }

    #[test]
    fn test_tradeable_serde_roundtrip() {
        let items = vec![
            Tradeable::Material("copper".to_string(), 10),
            Tradeable::Agent("miner_42".to_string(), 0.88),
            Tradeable::Knowledge("archery".to_string(), 0.95),
            Tradeable::Recipe("potion".to_string()),
            Tradeable::Pet("phoenix".to_string(), 99),
        ];

        for item in &items {
            let json = serde_json::to_string(item).unwrap();
            let deserialized: Tradeable = serde_json::from_str(&json).unwrap();
            assert_eq!(*item, deserialized);
        }
    }

    #[test]
    fn test_market_serde_roundtrip() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Agent("builder".to_string(), 0.92)],
            vec![Tradeable::Material("gold".to_string(), 100)],
        );
        market.accept(&id).unwrap();

        let json = serde_json::to_string(&market).unwrap();
        let deserialized: TradeMarket = serde_json::from_str(&json).unwrap();
        assert_eq!(market.history.len(), deserialized.history.len());
        assert_eq!(
            market.history[0].offer.from,
            deserialized.history[0].offer.from
        );
    }

    #[test]
    fn test_new_market_is_empty() {
        let market = TradeMarket::new();
        assert!(market.offers.is_empty());
        assert!(market.history.is_empty());
    }

    #[test]
    fn test_default_market_is_empty() {
        let market = TradeMarket::default();
        assert!(market.offers.is_empty());
        assert!(market.history.is_empty());
    }

    #[test]
    fn test_multiple_offers_find_offers_for() {
        let mut market = TradeMarket::new();
        market.create_offer("alice", "bob", vec![Tradeable::Material("wood".into(), 5)], vec![]);
        market.create_offer("carol", "bob", vec![Tradeable::Pet("hamster".into(), 1)], vec![]);
        market.create_offer("dave", "bob", vec![Tradeable::Knowledge("coding".into(), 0.6)], vec![]);

        let offers = market.find_offers_for("bob");
        assert_eq!(offers.len(), 3);
    }

    #[test]
    fn test_accept_moves_to_history_and_clears_offers() {
        let mut market = TradeMarket::new();
        let id = market.create_offer(
            "alice",
            "bob",
            vec![Tradeable::Recipe("cake".into())],
            vec![Tradeable::Recipe("pie".into())],
        );
        market.accept(&id).unwrap();
        assert!(market.offers.is_empty());
        assert_eq!(market.history.len(), 1);
    }

    #[test]
    fn test_gift_includes_message() {
        let mut market = TradeMarket::new();
        let gift = Gift {
            from: "eve".to_string(),
            to: "frank".to_string(),
            item: Tradeable::Knowledge("alchemy".to_string(), 0.99),
            message: "For your research".to_string(),
        };
        let msg = gift.message.clone();
        market.send_gift(gift);
        let ct = &market.history[0];
        assert_eq!(msg, "For your research");
        assert_eq!(ct.offer.from, "eve");
        assert_eq!(ct.offer.to, "frank");
    }
}
