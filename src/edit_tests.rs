use super::*;

#[test]
fn from_length_creates_single_region() {
    let el = EditList::from_length(100);
    assert_eq!(el.total_frames(), 100);
    assert_eq!(el.regions.len(), 1);
}

#[test]
fn ripple_delete_middle() {
    let mut el = EditList::from_length(100);
    el.ripple_delete(20, 30);
    assert_eq!(el.total_frames(), 90);
    assert_eq!(el.resolve(20), Some(30));
    assert_eq!(el.resolve(19), Some(19));
    assert_eq!(el.resolve(89), Some(99));
    assert_eq!(el.resolve(90), None);
}

#[test]
fn ripple_delete_at_boundaries() {
    let mut el = EditList::from_length(50);
    el.ripple_delete(0, 10);
    assert_eq!(el.total_frames(), 40);
    assert_eq!(el.resolve(0), Some(10));

    let mut el2 = EditList::from_length(50);
    el2.ripple_delete(40, 50);
    assert_eq!(el2.total_frames(), 40);
    assert_eq!(el2.resolve(39), Some(39));
    assert_eq!(el2.resolve(40), None);
}

#[test]
fn crop_keeps_range() {
    let mut el = EditList::from_length(100);
    el.crop(10, 40);
    assert_eq!(el.total_frames(), 30);
    assert_eq!(el.resolve(0), Some(10));
    assert_eq!(el.resolve(29), Some(39));
    assert_eq!(el.resolve(30), None);
}

#[test]
fn iter_source_frames_full() {
    let el = EditList::from_length(5);
    let frames: Vec<Option<usize>> = el.iter_source_frames(0, 5).collect();
    assert_eq!(frames, vec![Some(0), Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn iter_source_frames_after_delete() {
    let mut el = EditList::from_length(100);
    el.ripple_delete(20, 30);
    let frames: Vec<Option<usize>> = el.iter_source_frames(18, 5).collect();
    assert_eq!(frames, vec![Some(18), Some(19), Some(30), Some(31), Some(32)]);
}

#[test]
fn iter_source_frames_partial() {
    let el = EditList::from_length(10);
    let frames: Vec<Option<usize>> = el.iter_source_frames(3, 4).collect();
    assert_eq!(frames, vec![Some(3), Some(4), Some(5), Some(6)]);
}

#[test]
fn gap_delete_replaces_with_silence() {
    let mut el = EditList::from_length(100);
    el.gap_delete(20, 30);
    assert_eq!(el.total_frames(), 100);
    assert_eq!(el.resolve(19), Some(19));
    assert_eq!(el.resolve(20), None);
    assert_eq!(el.resolve(29), None);
    assert_eq!(el.resolve(30), Some(30));
    assert_eq!(el.resolve(99), Some(99));
}

#[test]
fn gap_delete_preserves_total_length() {
    let mut el = EditList::from_length(50);
    el.gap_delete(10, 40);
    assert_eq!(el.total_frames(), 50);
}

#[test]
fn is_silence_range_source_returns_false() {
    let el = EditList::from_length(100);
    assert!(!el.is_silence_range(10, 20));
}

#[test]
fn is_silence_range_after_gap_delete() {
    let mut el = EditList::from_length(100);
    el.gap_delete(20, 40);
    assert!(el.is_silence_range(20, 40));
    assert!(!el.is_silence_range(19, 40));
    assert!(!el.is_silence_range(20, 41));
    assert!(el.is_silence_range(25, 35));
}

#[test]
fn iter_source_frames_with_silence() {
    let mut el = EditList::from_length(5);
    el.gap_delete(2, 4);
    let frames: Vec<Option<usize>> = el.iter_source_frames(0, 5).collect();
    assert_eq!(frames, vec![Some(0), Some(1), None, None, Some(4)]);
}

#[test]
fn extract_regions_empty_range() {
    let el = EditList::from_length(100);
    assert!(el.extract_regions(10, 10).is_empty());
    assert!(el.extract_regions(20, 10).is_empty());
}

#[test]
fn extract_regions_full() {
    let el = EditList::from_length(100);
    let regions = el.extract_regions(0, 100);
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].len(), 100);
    if let Region::Source { source_start, source_end } = regions[0] {
        assert_eq!(source_start, 0);
        assert_eq!(source_end, 100);
    } else {
        panic!("expected Source region");
    }
}

#[test]
fn extract_regions_subset() {
    let el = EditList::from_length(100);
    let regions = el.extract_regions(20, 50);
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].len(), 30);
    if let Region::Source { source_start, source_end } = regions[0] {
        assert_eq!(source_start, 20);
        assert_eq!(source_end, 50);
    } else {
        panic!("expected Source region");
    }
}

#[test]
fn extract_regions_across_boundary() {
    let mut el = EditList::from_length(100);
    el.ripple_delete(40, 60);
    let regions = el.extract_regions(30, 50);
    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].len(), 10);
    assert_eq!(regions[1].len(), 10);
    if let Region::Source { source_start, source_end } = regions[0] {
        assert_eq!(source_start, 30);
        assert_eq!(source_end, 40);
    } else {
        panic!("expected Source region");
    }
    if let Region::Source { source_start, source_end } = regions[1] {
        assert_eq!(source_start, 60);
        assert_eq!(source_end, 70);
    } else {
        panic!("expected Source region");
    }
}

#[test]
fn insert_at_start() {
    let mut el = EditList::from_length(100);
    let patch = vec![Region::Source { source_start: 50, source_end: 60 }];
    el.insert(0, &patch);
    assert_eq!(el.total_frames(), 110);
    assert_eq!(el.resolve(0), Some(50));
    assert_eq!(el.resolve(9), Some(59));
    assert_eq!(el.resolve(10), Some(0));
}

#[test]
fn insert_at_end() {
    let mut el = EditList::from_length(100);
    let patch = vec![Region::Source { source_start: 0, source_end: 10 }];
    el.insert(100, &patch);
    assert_eq!(el.total_frames(), 110);
    assert_eq!(el.resolve(99), Some(99));
    assert_eq!(el.resolve(100), Some(0));
    assert_eq!(el.resolve(109), Some(9));
}

#[test]
fn insert_in_middle() {
    let mut el = EditList::from_length(100);
    let patch = vec![Region::Silence { len: 5 }];
    el.insert(50, &patch);
    assert_eq!(el.total_frames(), 105);
    assert_eq!(el.resolve(49), Some(49));
    assert_eq!(el.resolve(50), None);
    assert_eq!(el.resolve(54), None);
    assert_eq!(el.resolve(55), Some(50));
    assert_eq!(el.resolve(104), Some(99));
}

#[test]
fn insert_empty_is_noop() {
    let mut el = EditList::from_length(100);
    el.insert(50, &[]);
    assert_eq!(el.total_frames(), 100);
}
