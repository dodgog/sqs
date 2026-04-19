use std::collections::HashSet;

/// Generate a random ID in the format [a-z]{4}[0-9]{4} (e.g., "abcd1234").
/// Retries on collision with existing IDs.
pub fn generate_id(existing: &HashSet<String>) -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    loop {
        let letters: String = (0..4)
            .map(|_| (rng.random_range(0u8..26) + b'a') as char)
            .collect();
        let digits: String = (0..4)
            .map(|_| (rng.random_range(0u8..10) + b'0') as char)
            .collect();
        let id = format!("{letters}{digits}");
        if !existing.contains(&id) {
            return id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correct_format() {
        let id = generate_id(&HashSet::new());
        assert_eq!(id.len(), 8);
        assert!(id[..4].chars().all(|c| c.is_ascii_lowercase()));
        assert!(id[4..].chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn avoids_collisions() {
        let mut existing = HashSet::new();
        existing.insert("abcd1234".to_string());
        let id = generate_id(&existing);
        assert_ne!(id, "abcd1234");
        assert_eq!(id.len(), 8);
    }

    #[test]
    fn generates_unique_ids() {
        let existing = HashSet::new();
        let mut generated = HashSet::new();
        for _ in 0..10_000 {
            let id = generate_id(&existing);
            assert!(generated.insert(id.clone()), "duplicate id generated: {id}");
        }
    }
}
