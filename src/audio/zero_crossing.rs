use crate::edit::EditList;

pub fn find_nearest_zero_crossing(
    samples: &[f32],
    edit_list: &EditList,
    edit_frame: usize,
    search_radius: usize,
) -> usize {
    let total = edit_list.total_frames();
    for delta in 0..search_radius {
        for &candidate in &[
            edit_frame.saturating_sub(delta),
            (edit_frame + delta).min(total.saturating_sub(1)),
        ] {
            let Some((src, _)) = edit_list.resolve(candidate) else { continue };
            let Some((src_prev, _)) = edit_list.resolve(candidate.saturating_sub(1)) else { continue };
            let a = samples.get(src_prev).copied().unwrap_or(0.0);
            let b = samples.get(src).copied().unwrap_or(0.0);
            if a * b <= 0.0 {
                return candidate;
            }
        }
    }
    edit_frame
}
