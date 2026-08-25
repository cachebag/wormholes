//! Oracle test: every operation sequence must behave identically to
//! std's BTreeMap.

use proptest::prelude::*;
use std::collections::BTreeMap;
use wormholes::Wormholes;

#[derive(Debug, Clone)]
enum Op {
    Insert(Vec<u8>, u64),
    Get(Vec<u8>),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    let key = proptest::collection::vec(0u8..4, 0..6);
    prop_oneof![
        (key.clone(), any::<u64>()).prop_map(|(k, v)| Op::Insert(k, v)),
        key.prop_map(Op::Get),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    fn matches_btreemap(ops in proptest::collection::vec(op_strategy(), 1..400)) {
        let mut wh: Wormholes<u64> = Wormholes::new();
        let mut oracle: BTreeMap<Vec<u8>, u64> = BTreeMap::new();

        for op in ops {
            match op {
                Op::Insert(k, v) => {
                    prop_assert_eq!(wh.insert(&k, v), oracle.insert(k, v));
                }
                Op::Get(k) => {
                    prop_assert_eq!(wh.get(&k), oracle.get(&k));
                }
            }
        }

        let got: Vec<_> = wh.iter().map(|(k, v)| (k.to_vec(), *v)).collect();
        let want: Vec<_> = oracle.into_iter().collect();
        prop_assert_eq!(got, want);
    }
}
