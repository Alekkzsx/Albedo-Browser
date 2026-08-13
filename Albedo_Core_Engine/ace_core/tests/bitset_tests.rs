use ace_core::bitset::BitSet;

#[test]
fn test_bitset_basic_operations() {
    let mut bitset = BitSet::new();

    assert!(!bitset.contains(0));
    assert!(!bitset.contains(63));
    assert!(!bitset.contains(64));

    bitset.insert(0);
    bitset.insert(63);
    bitset.insert(64);
    bitset.insert(1000);

    assert!(bitset.contains(0));
    assert!(bitset.contains(63));
    assert!(bitset.contains(64));
    assert!(bitset.contains(1000));
    assert!(!bitset.contains(1001));

    assert_eq!(bitset.count_ones(), 4);

    bitset.remove(64);
    assert!(!bitset.contains(64));
    assert_eq!(bitset.count_ones(), 3);

    bitset.clear();
    assert_eq!(bitset.count_ones(), 0);
    assert!(!bitset.contains(0));
}

#[test]
fn test_bitset_union_intersection() {
    let mut a = BitSet::new();
    a.insert(10);
    a.insert(20);
    a.insert(200);

    let mut b = BitSet::new();
    b.insert(20);
    b.insert(30);

    let mut intersection = a.clone();
    intersection.intersection_with(&b);
    assert!(!intersection.contains(10));
    assert!(intersection.contains(20));
    assert!(!intersection.contains(30));
    assert!(!intersection.contains(200));
    assert_eq!(intersection.count_ones(), 1);

    let mut union = a.clone();
    union.union_with(&b);
    assert!(union.contains(10));
    assert!(union.contains(20));
    assert!(union.contains(30));
    assert!(union.contains(200));
    assert_eq!(union.count_ones(), 4);
}
