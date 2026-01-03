#[cfg(test)]
mod tests {
    use crate::plugins::metagame::{PlayerStats, GlobalTime, SavedItem, SaveData};
    use serde_json;

    #[test]
    fn test_save_serialization() {
        let stats = PlayerStats {
            thalers: 500,
            reputation: 10,
            infection: 90,
        };
        let time = GlobalTime { day: 2, hour: 12 };
        let inventory = vec![
            SavedItem {
                item_id: "test_sword".to_string(),
                grid_x: 0,
                grid_y: 0,
            }
        ];

        let save_data = SaveData {
            player_stats: stats.clone(),
            global_time: time.clone(),
            inventory: inventory.clone(),
        };

        let json = serde_json::to_string(&save_data).unwrap();

        let loaded: SaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.player_stats.thalers, 500);
        assert_eq!(loaded.global_time.day, 2);
        assert_eq!(loaded.inventory[0].item_id, "test_sword");
    }
}
