use crate::adapter::ListDef;

pub const LIST_SPACING: f64 = 1000.0;
pub const FIRST_LIST_KEY: f64 = LIST_SPACING;
pub const EPSILON: f64 = 1e-9;

pub fn list_key_for_index(index: usize) -> f64 {
    (index as f64 + 1.0) * LIST_SPACING
}

pub fn item_key_in_list(list_key: f64, position_within_list: usize) -> f64 {
    list_key + 1.0 + position_within_list as f64
}

pub fn midpoint(a: f64, b: f64) -> Option<f64> {
    let mid = (a + b) / 2.0;
    if mid - a < EPSILON || b - mid < EPSILON {
        None
    } else {
        Some(mid)
    }
}

pub fn derive_list_for_order(order_key: f64, lists_sorted: &[ListDef]) -> Option<&str> {
    lists_sorted
        .iter()
        .rfind(|l| l.order <= order_key)
        .map(|l| l.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lists() -> Vec<ListDef> {
        vec![
            ListDef {
                name: "now".into(),
                display: "Now".into(),
                order: 1000.0,
                tags: Vec::new(),
            },
            ListDef {
                name: "next".into(),
                display: "Next".into(),
                order: 2000.0,
                tags: Vec::new(),
            },
            ListDef {
                name: "done".into(),
                display: "Done".into(),
                order: 3000.0,
                tags: Vec::new(),
            },
        ]
    }

    #[test]
    fn list_key_spacing_is_thousand() {
        assert_eq!(list_key_for_index(0), 1000.0);
        assert_eq!(list_key_for_index(1), 2000.0);
        assert_eq!(list_key_for_index(4), 5000.0);
    }

    #[test]
    fn item_key_starts_just_after_list_marker() {
        assert_eq!(item_key_in_list(1000.0, 0), 1001.0);
        assert_eq!(item_key_in_list(1000.0, 9), 1010.0);
    }

    #[test]
    fn midpoint_returns_value_inside_gap() {
        assert_eq!(midpoint(1001.0, 1002.0), Some(1001.5));
    }

    #[test]
    fn midpoint_refuses_underflow() {
        assert_eq!(midpoint(1.0, 1.0 + EPSILON / 2.0), None);
    }

    #[test]
    fn derive_list_picks_nearest_preceding_marker() {
        let ls = lists();
        assert_eq!(derive_list_for_order(1000.0, &ls), Some("now"));
        assert_eq!(derive_list_for_order(1500.0, &ls), Some("now"));
        assert_eq!(derive_list_for_order(2000.0, &ls), Some("next"));
        assert_eq!(derive_list_for_order(2999.0, &ls), Some("next"));
        assert_eq!(derive_list_for_order(3000.0, &ls), Some("done"));
        assert_eq!(derive_list_for_order(9999.0, &ls), Some("done"));
    }

    #[test]
    fn derive_list_returns_none_when_below_first_marker() {
        let ls = lists();
        assert_eq!(derive_list_for_order(500.0, &ls), None);
    }
}
