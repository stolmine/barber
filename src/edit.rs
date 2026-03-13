const DEFAULT_FADE_LEN: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FadeCurve {
    Linear,
    Exponential,
    Logarithmic,
    SCurve,
}

impl FadeCurve {
    pub fn apply(self, t: f32) -> f32 {
        match self {
            FadeCurve::Linear => t,
            FadeCurve::Exponential => t * t,
            FadeCurve::Logarithmic => t.sqrt(),
            FadeCurve::SCurve => 3.0 * t * t - 2.0 * t * t * t,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RegionKind {
    Source { source_start: usize, source_end: usize },
    Silence { len: usize },
    Reversed { source_start: usize, source_end: usize },
}

#[derive(Clone, Debug)]
pub struct Region {
    pub kind: RegionKind,
    pub gain: f32,
    pub dc_offset: f32,
    pub fade_in: usize,
    pub fade_out: usize,
    pub fade_in_curve: FadeCurve,
    pub fade_out_curve: FadeCurve,
}

impl Region {
    pub fn source(source_start: usize, source_end: usize) -> Self {
        Self { kind: RegionKind::Source { source_start, source_end }, gain: 1.0, dc_offset: 0.0, fade_in: 0, fade_out: 0, fade_in_curve: FadeCurve::Linear, fade_out_curve: FadeCurve::Linear }
    }
    pub fn silence(len: usize) -> Self {
        Self { kind: RegionKind::Silence { len }, gain: 1.0, dc_offset: 0.0, fade_in: 0, fade_out: 0, fade_in_curve: FadeCurve::Linear, fade_out_curve: FadeCurve::Linear }
    }
    pub fn reversed(source_start: usize, source_end: usize) -> Self {
        Self { kind: RegionKind::Reversed { source_start, source_end }, gain: 1.0, dc_offset: 0.0, fade_in: 0, fade_out: 0, fade_in_curve: FadeCurve::Linear, fade_out_curve: FadeCurve::Linear }
    }

    pub fn len(&self) -> usize {
        match &self.kind {
            RegionKind::Source { source_start, source_end } => source_end - source_start,
            RegionKind::Silence { len } => *len,
            RegionKind::Reversed { source_start, source_end } => source_end - source_start,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditList {
    regions: Vec<Region>,
    pub fades_enabled: bool,
}

impl EditList {
    pub fn from_length(num_frames: usize) -> Self {
        Self {
            regions: vec![Region::source(0, num_frames)],
            fades_enabled: true,
        }
    }

    pub fn total_frames(&self) -> usize {
        self.regions.iter().map(|r| r.len()).sum()
    }

    pub fn resolve(&self, edit_frame: usize) -> Option<(usize, f32, f32)> {
        let mut offset = 0;
        for region in &self.regions {
            let rlen = region.len();
            if edit_frame < offset + rlen {
                let pos = edit_frame - offset;
                let fade_gain = if self.fades_enabled {
                    let fade_in_env = if region.fade_in > 0 && pos < region.fade_in {
                        region.fade_in_curve.apply(pos as f32 / region.fade_in as f32)
                    } else { 1.0 };
                    let fade_out_env = if region.fade_out > 0 && pos >= rlen - region.fade_out {
                        region.fade_out_curve.apply((rlen - 1 - pos) as f32 / region.fade_out as f32)
                    } else { 1.0 };
                    fade_in_env * fade_out_env
                } else { 1.0 };
                let effective_gain = region.gain * fade_gain;
                return match &region.kind {
                    RegionKind::Source { source_start, .. } => Some((source_start + pos, effective_gain, region.dc_offset)),
                    RegionKind::Silence { .. } => None,
                    RegionKind::Reversed { source_end, .. } => Some((source_end - 1 - pos, effective_gain, region.dc_offset)),
                };
            }
            offset += rlen;
        }
        None
    }

    pub fn iter_source_frames(&self, start: usize, len: usize) -> impl Iterator<Item = Option<(usize, f32, f32)>> + '_ {
        let end = start + len;
        let mut offset = 0usize;
        let mut region_idx = 0usize;

        while region_idx < self.regions.len() {
            let rlen = self.regions[region_idx].len();
            if offset + rlen > start {
                break;
            }
            offset += rlen;
            region_idx += 1;
        }

        let mut current_idx = region_idx;
        let mut current_offset = offset;
        let mut edit_pos = start;

        std::iter::from_fn(move || {
            if edit_pos >= end {
                return None;
            }
            loop {
                if current_idx >= self.regions.len() {
                    return None;
                }
                let region = &self.regions[current_idx];
                let rlen = region.len();
                if edit_pos < current_offset + rlen {
                    let pos = edit_pos - current_offset;
                    let fade_gain = if self.fades_enabled {
                        let fade_in_env = if region.fade_in > 0 && pos < region.fade_in {
                            region.fade_in_curve.apply(pos as f32 / region.fade_in as f32)
                        } else { 1.0 };
                        let fade_out_env = if region.fade_out > 0 && pos >= rlen - region.fade_out {
                            region.fade_out_curve.apply((rlen - 1 - pos) as f32 / region.fade_out as f32)
                        } else { 1.0 };
                        fade_in_env * fade_out_env
                    } else { 1.0 };
                    let effective_gain = region.gain * fade_gain;
                    let result = match &region.kind {
                        RegionKind::Source { source_start, .. } => Some(Some((source_start + pos, effective_gain, region.dc_offset))),
                        RegionKind::Silence { .. } => Some(None),
                        RegionKind::Reversed { source_end, .. } => Some(Some((source_end - 1 - pos, effective_gain, region.dc_offset))),
                    };
                    edit_pos += 1;
                    return result;
                }
                current_offset += rlen;
                current_idx += 1;
            }
        })
    }

    pub fn is_silence_range(&self, start: usize, end: usize) -> bool {
        if start >= end {
            return false;
        }
        let mut offset = 0usize;
        let mut checked = start;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            if r_end <= start {
                offset = r_end;
                continue;
            }
            if r_start >= end {
                break;
            }
            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);
            if overlap_start < overlap_end {
                if !matches!(region.kind, RegionKind::Silence { .. }) {
                    return false;
                }
                checked = overlap_end;
            }
            offset = r_end;
        }
        checked >= end
    }

    fn transform_regions<F>(&mut self, start: usize, end: usize, mut emit: F)
    where
        F: FnMut(&Region, usize, usize, usize, usize, &mut Vec<Region>),
    {
        let mut new_regions: Vec<Region> = Vec::new();
        let mut offset = 0usize;

        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);
            emit(region, overlap_start, overlap_end, r_start, r_end, &mut new_regions);
            offset = r_end;
        }

        self.regions = new_regions;
    }

    fn ripple_delete_inner(&mut self, start: usize, end: usize, apply_boundary: bool) {
        if start >= end {
            return;
        }
        self.transform_regions(start, end, |region, overlap_start, overlap_end, r_start, r_end, out| {
            if overlap_start < overlap_end {
                let pre_len = overlap_start - r_start;
                let post_len = r_end - overlap_end;
                if pre_len > 0 {
                    out.push(split_region_prefix(region, pre_len));
                }
                if post_len > 0 {
                    out.push(split_region_suffix(region, overlap_end - r_start, r_end - r_start));
                }
            } else {
                out.push(region.clone());
            }
        });
        if apply_boundary {
            self.apply_boundary_fades_at(start);
        }
    }

    pub fn ripple_delete(&mut self, start: usize, end: usize) {
        self.ripple_delete_inner(start, end, true);
    }

    pub fn crop(&mut self, start: usize, end: usize) {
        if start >= end {
            self.regions.clear();
            return;
        }
        self.transform_regions(start, end, |region, overlap_start, overlap_end, r_start, _r_end, out| {
            if overlap_start < overlap_end {
                let inner_start = overlap_start - r_start;
                let inner_end = overlap_end - r_start;
                out.push(split_region_suffix(region, inner_start, inner_end));
            }
        });
    }

    pub fn extract_regions(&self, start: usize, end: usize) -> Vec<Region> {
        if start >= end {
            return Vec::new();
        }
        let mut result = Vec::new();
        let mut offset = 0usize;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);
            if overlap_start < overlap_end {
                let inner_start = overlap_start - r_start;
                let inner_end = overlap_end - r_start;
                result.push(split_region_suffix(region, inner_start, inner_end));
            }
            offset = r_end;
        }
        result
    }

    fn insert_inner(&mut self, position: usize, regions: &[Region], apply_boundary: bool) {
        if regions.is_empty() {
            return;
        }
        let insert_len: usize = regions.iter().map(|r| r.len()).sum();
        let mut new_regions = Vec::new();
        let mut offset = 0usize;
        let mut inserted = false;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            if !inserted && position <= r_end && position >= r_start {
                let pre_len = position - r_start;
                let post_len = r_end - position;
                if pre_len > 0 {
                    new_regions.push(split_region_prefix(region, pre_len));
                }
                new_regions.extend_from_slice(regions);
                inserted = true;
                if post_len > 0 {
                    new_regions.push(split_region_suffix(region, pre_len, rlen));
                }
            } else {
                new_regions.push(region.clone());
            }
            offset = r_end;
        }
        if !inserted {
            new_regions.extend_from_slice(regions);
        }
        self.regions = new_regions;
        if apply_boundary {
            self.apply_boundary_fades_at(position);
            self.apply_boundary_fades_at(position + insert_len);
        }
    }

    pub fn insert(&mut self, position: usize, regions: &[Region]) {
        self.insert_inner(position, regions, true);
    }

    pub fn gap_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let gap_len = end - start;
        let gap_inserted = &mut false;
        self.transform_regions(start, end, |region, overlap_start, overlap_end, r_start, r_end, out| {
            if overlap_start < overlap_end {
                let pre_len = overlap_start - r_start;
                let post_len = r_end - overlap_end;
                if pre_len > 0 {
                    out.push(split_region_prefix(region, pre_len));
                }
                if !*gap_inserted {
                    out.push(Region::silence(gap_len));
                    *gap_inserted = true;
                }
                if post_len > 0 {
                    out.push(split_region_suffix(region, overlap_end - r_start, r_end - r_start));
                }
            } else {
                out.push(region.clone());
            }
        });
    }

    pub fn reverse_selection(&mut self, start: usize, end: usize) {
        if start >= end { return; }
        let mut extracted = self.extract_regions(start, end);
        extracted.reverse();
        for region in &mut extracted {
            region.kind = match &region.kind {
                RegionKind::Source { source_start, source_end } => RegionKind::Reversed { source_start: *source_start, source_end: *source_end },
                RegionKind::Reversed { source_start, source_end } => RegionKind::Source { source_start: *source_start, source_end: *source_end },
                RegionKind::Silence { len } => RegionKind::Silence { len: *len },
            };
        }
        self.ripple_delete(start, end);
        self.insert(start, &extracted);
        self.apply_boundary_fades_at(start);
        self.apply_boundary_fades_at(end);
    }

    pub fn for_each_source_range(&self, edit_start: usize, edit_end: usize, mut f: impl FnMut(usize, usize, f32)) {
        let mut offset = 0;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            if r_end <= edit_start { offset = r_end; continue; }
            if r_start >= edit_end { break; }
            let overlap_start = edit_start.max(r_start);
            let overlap_end = edit_end.min(r_end);
            if overlap_start < overlap_end {
                let inner_start = overlap_start - r_start;
                let inner_end = overlap_end - r_start;
                let mid = (inner_start + inner_end) / 2;
                let fade_gain = if self.fades_enabled {
                    let fi = if region.fade_in > 0 && mid < region.fade_in {
                        region.fade_in_curve.apply(mid as f32 / region.fade_in as f32)
                    } else { 1.0 };
                    let fo = if region.fade_out > 0 && mid >= rlen - region.fade_out {
                        region.fade_out_curve.apply((rlen - 1 - mid) as f32 / region.fade_out as f32)
                    } else { 1.0 };
                    fi * fo
                } else { 1.0 };
                let gain = region.gain * fade_gain;
                match &region.kind {
                    RegionKind::Source { source_start, .. } => {
                        f(source_start + inner_start, source_start + inner_end, gain);
                    }
                    RegionKind::Reversed { source_end, .. } => {
                        f(source_end - inner_end, source_end - inner_start, gain);
                    }
                    RegionKind::Silence { .. } => {}
                }
            }
            offset = r_end;
        }
    }

    pub fn set_gain_range(&mut self, start: usize, end: usize, gain_factor: f32) {
        if start >= end { return; }
        let mut extracted = self.extract_regions(start, end);
        for region in &mut extracted {
            region.gain *= gain_factor;
        }
        self.ripple_delete(start, end);
        self.insert(start, &extracted);
    }

    pub fn set_dc_offset_range(&mut self, start: usize, end: usize, dc_offset: f32) {
        if start >= end { return; }
        let mut extracted = self.extract_regions(start, end);
        for region in &mut extracted {
            region.dc_offset += dc_offset;
        }
        self.ripple_delete(start, end);
        self.insert(start, &extracted);
    }

    pub fn apply_fade_in(&mut self, start: usize, end: usize, curve: FadeCurve) {
        if start >= end { return; }
        let fade_len = end - start;
        let mut extracted = self.extract_regions(start, end);
        for region in &mut extracted {
            let rlen = region.len();
            region.fade_in = fade_len.min(rlen);
            region.fade_in_curve = curve;
        }
        log::debug!("apply_fade_in: fade_len={}, regions={}, first_fade_in={}",
            fade_len, extracted.len(), extracted.first().map_or(0, |r| r.fade_in));
        self.ripple_delete_inner(start, end, false);
        self.insert_inner(start, &extracted, false);
    }

    pub fn apply_fade_out(&mut self, start: usize, end: usize, curve: FadeCurve) {
        if start >= end { return; }
        let fade_len = end - start;
        let mut extracted = self.extract_regions(start, end);
        for region in &mut extracted {
            let rlen = region.len();
            region.fade_out = fade_len.min(rlen);
            region.fade_out_curve = curve;
        }
        log::debug!("apply_fade_out: fade_len={}, regions={}, first_fade_out={}",
            fade_len, extracted.len(), extracted.first().map_or(0, |r| r.fade_out));
        self.ripple_delete_inner(start, end, false);
        self.insert_inner(start, &extracted, false);
    }

    pub fn apply_boundary_fades(&mut self, start_idx: usize, end_idx: usize) {
        let end_idx = end_idx.min(self.regions.len());
        let start_idx = start_idx.min(end_idx);
        for i in start_idx..end_idx {
            let rlen = self.regions[i].len();
            let max_fade = rlen / 2;
            if i == start_idx {
                self.regions[i].fade_in = DEFAULT_FADE_LEN.min(max_fade);
            }
            if i == end_idx - 1 {
                self.regions[i].fade_out = DEFAULT_FADE_LEN.min(max_fade);
            }
        }
    }

    fn apply_boundary_fades_at(&mut self, edit_frame: usize) {
        let mut offset = 0;
        for i in 0..self.regions.len() {
            let rlen = self.regions[i].len();
            if offset == edit_frame && i > 0 {
                let prev_len = self.regions[i - 1].len();
                self.regions[i - 1].fade_out = DEFAULT_FADE_LEN.min(prev_len / 2);
                self.regions[i].fade_in = DEFAULT_FADE_LEN.min(rlen / 2);
                return;
            }
            offset += rlen;
        }
    }

    fn region_index_at(&self, edit_frame: usize) -> Option<usize> {
        let mut offset = 0;
        for (i, region) in self.regions.iter().enumerate() {
            let rlen = region.len();
            if edit_frame < offset + rlen {
                return Some(i);
            }
            if edit_frame == offset + rlen {
                return Some((i + 1).min(self.regions.len() - 1));
            }
            offset += rlen;
        }
        None
    }
}

fn split_region_prefix(region: &Region, prefix_len: usize) -> Region {
    let kind = match &region.kind {
        RegionKind::Source { source_start, .. } => RegionKind::Source {
            source_start: *source_start,
            source_end: source_start + prefix_len,
        },
        RegionKind::Silence { .. } => RegionKind::Silence { len: prefix_len },
        RegionKind::Reversed { source_end, .. } => RegionKind::Reversed {
            source_start: source_end - prefix_len,
            source_end: *source_end,
        },
    };
    Region { kind, ..region.clone() }
}

fn split_region_suffix(region: &Region, inner_start: usize, inner_end: usize) -> Region {
    let kind = match &region.kind {
        RegionKind::Source { source_start, .. } => RegionKind::Source {
            source_start: source_start + inner_start,
            source_end: source_start + inner_end,
        },
        RegionKind::Silence { .. } => RegionKind::Silence { len: inner_end - inner_start },
        RegionKind::Reversed { source_end, .. } => RegionKind::Reversed {
            source_start: source_end - inner_end,
            source_end: source_end - inner_start,
        },
    };
    Region { kind, ..region.clone() }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
