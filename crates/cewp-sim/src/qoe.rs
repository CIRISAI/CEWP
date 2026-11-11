//! What a person in a call experiences, from the logs the media engine keeps per
//! (stream, receiver) session (`FSD/twin/media.md` §4).
//!
//! Every metric here is computed after the run from four facts per frame: when it was captured,
//! whether it was sent toward the receiver, whether all of its fragments were released by the
//! receiver's reorder window, and when (if ever) it was shown. Definitions:
//!
//! - **join time**: from the later of (receiver joined, publisher joined) to the first frame
//!   shown (necessarily a keyframe: a P-frame needs its chain).
//! - **glass-to-glass**: shown − captured + the capture/encode delay, per frame; per session the
//!   median at each tree depth.
//! - **frame loss**: of the frames sent toward the receiver, those not released whole (a frame
//!   is lost when any fragment is), by frame type.
//! - **freeze**: a gap between consecutive shown frames longer than
//!   `max(3 · interval, interval + 150 ms)` (the W3C webrtc-stats `freezeCount` rule), where the
//!   interval is the frame spacing of the layers the receiver asked for; the whole gap counts.
//! - **audio**: a 20 ms frame sent toward the receiver and not released within the audio
//!   deadline is missing; an audible gap is a run of two or more missing frames (Opus PLC
//!   covers one).
//! - **layer**: a one-second window is at the requested layer when at least half the frames of
//!   the requested top layer captured in it were shown.
//! - **relay recovery**: around a parent's departure at `L`, the audio and video gaps that span
//!   `L`, and the time from `L` to the first full-layer frame shown.

/// One video frame as the publisher produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VMeta {
    /// Capture time (us).
    pub capture_us: u64,
    /// An IDR keyframe.
    pub key: bool,
    /// On the enhancement temporal layer (t1); otherwise the base (t0).
    pub t1: bool,
    /// Fragments (chunks) it took.
    pub frags: u16,
    /// Encoded size (bytes).
    pub bytes: u32,
}

/// One receiver's view of one stream.
#[derive(Debug, Clone, Default)]
pub struct SessionLog {
    /// Publisher node.
    pub stream: u16,
    /// Receiver node.
    pub receiver: u16,
    /// From when this session counts: max(receiver join, publisher join) (us).
    pub start_us: u64,
    /// When it stops counting: receiver left, stream ended, or the run ended (us).
    pub end_us: u64,
    /// The receiver asked for the base layer only.
    pub requested_t0: bool,
    /// Video frame sent toward this receiver (by index).
    pub vid_sent: Vec<bool>,
    /// Video frame released whole.
    pub vid_complete: Vec<bool>,
    /// When the frame was shown (us; 0 = never).
    pub vid_shown: Vec<u64>,
    /// Tree depth of the receiver when the frame was shown.
    pub vid_depth: Vec<u8>,
    /// Audio frame sent toward this receiver.
    pub aud_sent: Vec<bool>,
    /// When the audio frame was released (us; 0 = never).
    pub aud_play: Vec<u64>,
    /// Unique chunks sent toward this receiver.
    pub chunks_sent: u64,
    /// Unique chunks it received.
    pub chunks_recv: u64,
}

/// Fixed delays and rules the metrics need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QoeRules {
    /// Video frames per second.
    pub fps: u32,
    /// Camera + encoder delay added to every shown frame (us).
    pub capture_encode_us: u64,
    /// An audio frame later than this after capture is missing (us).
    pub audio_deadline_us: u64,
    /// Frames captured in the last stretch of a session are in flight, not lost (us).
    pub tail_us: u64,
}

/// A parent left while this receiver took a stream through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryRec {
    /// Receiver.
    pub child: u16,
    /// Stream.
    pub stream: u16,
    /// When the parent left (us).
    pub leave_us: u64,
    /// The receiver held a warm standby on another parent.
    pub had_standby: bool,
    /// The parent left with a signed end (link closed) rather than vanishing.
    pub graceful: bool,
}

/// What a recovery cost the receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryObs {
    /// The record.
    pub rec: RecoveryRec,
    /// Audio gap spanning the departure (us); `None` when audio never resumed.
    pub audio_gap_us: Option<u64>,
    /// Video gap spanning the departure (us); `None` when video never resumed.
    pub video_gap_us: Option<u64>,
    /// From the departure to the first frame of the requested top layer (us).
    pub full_restore_us: Option<u64>,
}

/// One session's experience.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionQoe {
    /// Publisher.
    pub stream: u16,
    /// Receiver.
    pub receiver: u16,
    /// Join to first shown frame (us); `None` = never saw video.
    pub join_us: Option<u64>,
    /// Median glass-to-glass latency per depth 1..=4 (us).
    pub g2g_median_by_depth: [Option<u64>; 4],
    /// All glass-to-glass samples (us), for exact checks.
    pub g2g_samples: Vec<u64>,
    /// Audio release latency samples, capture to release (us).
    pub audio_lat_samples: Vec<u64>,
    /// Keyframes sent / lost.
    pub key_sent: u64,
    /// Keyframes lost.
    pub key_lost: u64,
    /// Base-layer P-frames sent.
    pub p0_sent: u64,
    /// Base-layer P-frames lost.
    pub p0_lost: u64,
    /// Enhancement frames sent.
    pub p1_sent: u64,
    /// Enhancement frames lost.
    pub p1_lost: u64,
    /// Frames sent but complete and still not decodable (broken chain).
    pub undecodable: u64,
    /// Freezes.
    pub freezes: u64,
    /// Total freeze time (us).
    pub freeze_us: u64,
    /// Measured span (us).
    pub span_us: u64,
    /// Audio frames sent.
    pub audio_sent: u64,
    /// Audio frames missing (lost or late).
    pub audio_missing: u64,
    /// Audible gaps (runs of >= 2 missing frames).
    pub audio_gaps: u64,
    /// One-second windows measured.
    pub layer_windows: u64,
    /// Windows at the requested layer.
    pub layer_ok_windows: u64,
    /// Unique chunk loss on the receiver's paths (ppm).
    pub chunk_loss_ppm: u64,
}

/// Frame loss per fragment-count bucket, measured against the closed form.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AmpBucket {
    /// Smallest fragment count in the bucket.
    pub frags_lo: u16,
    /// Largest fragment count in the bucket.
    pub frags_hi: u16,
    /// Frames sent.
    pub sent: u64,
    /// Frames lost.
    pub lost: u64,
    /// Sum over frames of the closed-form loss `1 - (1 - p)^n` with the session's chunk loss p.
    pub closed_form_sum: f64,
    /// Sum over frames of the closed form's variance `q(1-q)`.
    pub closed_form_var: f64,
}

/// The fragment-count buckets of [`AmpBucket`].
pub const AMP_EDGES: [u16; 9] = [1, 2, 4, 8, 16, 32, 64, 128, 4096];

fn depth_slot(d: u8) -> usize {
    usize::from(d.clamp(1, 4)) - 1
}

fn median(v: &mut [u64]) -> Option<u64> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    Some(v[(v.len() - 1) / 2])
}

/// Nearest-rank percentile (`pm` in permille) of an unsorted sample.
#[must_use]
pub fn percentile(v: &[u64], pm: u64) -> Option<u64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    let rank = (pm * s.len() as u64).div_ceil(1000).max(1) as usize;
    Some(s[rank.min(s.len()) - 1])
}

/// p50 / p95 / p99 of a sample.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dist {
    /// Sample size.
    pub n: usize,
    /// Median.
    pub p50: u64,
    /// 95th percentile.
    pub p95: u64,
    /// 99th percentile.
    pub p99: u64,
}

impl Dist {
    /// The distribution of `v`.
    #[must_use]
    pub fn of(v: &[u64]) -> Self {
        Self {
            n: v.len(),
            p50: percentile(v, 500).unwrap_or(0),
            p95: percentile(v, 950).unwrap_or(0),
            p99: percentile(v, 990).unwrap_or(0),
        }
    }
}

/// Score one session.
#[must_use]
pub fn score_session(log: &SessionLog, frames: &[VMeta], aframes: &[u64], rules: &QoeRules, amp: &mut [AmpBucket]) -> SessionQoe {
    let mut q = SessionQoe { stream: log.stream, receiver: log.receiver, ..SessionQoe::default() };
    let end_measure = log.end_us.saturating_sub(rules.tail_us);
    q.chunk_loss_ppm = if log.chunks_sent == 0 {
        0
    } else {
        log.chunks_sent.saturating_sub(log.chunks_recv) * 1_000_000 / log.chunks_sent
    };
    let p = q.chunk_loss_ppm as f64 / 1e6;
    let shown_at = |i: usize| log.vid_shown.get(i).copied().unwrap_or(0);
    let first_shown = (0..frames.len()).find(|&i| shown_at(i) != 0);
    q.join_us = first_shown.map(|i| shown_at(i).saturating_sub(log.start_us));
    let first_capture = first_shown.map_or(u64::MAX, |i| frames[i].capture_us);

    // Glass-to-glass by depth.
    let mut by_depth: [Vec<u64>; 4] = Default::default();
    for (i, f) in frames.iter().enumerate() {
        let t = shown_at(i);
        if t != 0 && f.capture_us <= end_measure {
            let lat = t - f.capture_us + rules.capture_encode_us;
            by_depth[depth_slot(log.vid_depth.get(i).copied().unwrap_or(1))].push(lat);
            q.g2g_samples.push(lat);
        }
    }
    for (slot, v) in by_depth.iter_mut().enumerate() {
        q.g2g_median_by_depth[slot] = median(v);
    }

    // Frame loss and amplification, from the first shown frame on.
    for (i, f) in frames.iter().enumerate() {
        if f.capture_us < first_capture || f.capture_us > end_measure || !log.vid_sent.get(i).copied().unwrap_or(false) {
            continue;
        }
        let complete = log.vid_complete.get(i).copied().unwrap_or(false);
        let lost = u64::from(!complete);
        if f.key {
            q.key_sent += 1;
            q.key_lost += lost;
        } else if f.t1 {
            q.p1_sent += 1;
            q.p1_lost += lost;
        } else {
            q.p0_sent += 1;
            q.p0_lost += lost;
        }
        if complete && shown_at(i) == 0 {
            q.undecodable += 1;
        }
        if let Some(b) = amp.iter_mut().find(|b| f.frags >= b.frags_lo && f.frags <= b.frags_hi) {
            let cf = 1.0 - (1.0 - p).powi(i32::from(f.frags));
            b.sent += 1;
            b.lost += lost;
            b.closed_form_sum += cf;
            b.closed_form_var += cf * (1.0 - cf);
        }
    }

    // Freezes over the shown frames, closing at the end of the span.
    if let Some(first) = first_shown {
        let step = 1_000_000 / u64::from(rules.fps.max(1)) * if log.requested_t0 { 2 } else { 1 };
        let bar = (3 * step).max(step + 150_000);
        let mut last = shown_at(first);
        for i in first + 1..frames.len() {
            if frames[i].capture_us > end_measure {
                break;
            }
            let t = shown_at(i);
            if t == 0 {
                continue;
            }
            let gap = t.saturating_sub(last);
            if gap > bar {
                q.freezes += 1;
                q.freeze_us += gap;
            }
            last = last.max(t);
        }
        let tail_gap = end_measure.saturating_sub(last);
        if end_measure > last && tail_gap > bar {
            q.freezes += 1;
            q.freeze_us += tail_gap;
        }
        q.span_us = end_measure.saturating_sub(shown_at(first));
    }

    // Audio.
    let mut run = 0u64;
    let a_start = log.start_us + rules.tail_us;
    for (i, &cap) in aframes.iter().enumerate() {
        if cap < a_start || cap > end_measure || !log.aud_sent.get(i).copied().unwrap_or(false) {
            continue;
        }
        q.audio_sent += 1;
        let t = log.aud_play.get(i).copied().unwrap_or(0);
        if t != 0 {
            q.audio_lat_samples.push(t - cap);
        }
        let ok = t != 0 && t - cap <= rules.audio_deadline_us;
        if ok {
            if run >= 2 {
                q.audio_gaps += 1;
            }
            run = 0;
        } else {
            q.audio_missing += 1;
            run += 1;
        }
    }
    if run >= 2 {
        q.audio_gaps += 1;
    }

    // Layer windows.
    if let Some(first) = first_shown {
        let w0 = shown_at(first);
        let mut w = w0;
        while w + 1_000_000 <= end_measure {
            let (mut want, mut got) = (0u64, 0u64);
            for (i, f) in frames.iter().enumerate() {
                if f.capture_us < w || f.capture_us >= w + 1_000_000 {
                    continue;
                }
                let top = if log.requested_t0 { !f.t1 } else { f.t1 };
                if top {
                    want += 1;
                    got += u64::from(shown_at(i) != 0);
                }
            }
            if want > 0 {
                q.layer_windows += 1;
                q.layer_ok_windows += u64::from(got * 2 >= want);
            }
            w += 1_000_000;
        }
    }
    q
}

/// Score a parent's departure for one receiver.
#[must_use]
pub fn score_recovery(rec: RecoveryRec, log: &SessionLog, frames: &[VMeta]) -> RecoveryObs {
    let l = rec.leave_us;
    let gap = |times: &[u64]| -> Option<u64> {
        let before = times.iter().copied().filter(|&t| t != 0 && t <= l).max()?;
        let after = times.iter().copied().filter(|&t| t > l).min()?;
        Some(after - before)
    };
    let full_restore_us = frames
        .iter()
        .enumerate()
        .filter(|(_, f)| if log.requested_t0 { !f.t1 } else { f.t1 })
        .filter_map(|(i, _)| log.vid_shown.get(i).copied().filter(|&t| t > l))
        .min()
        .map(|t| t - l);
    RecoveryObs { rec, audio_gap_us: gap(&log.aud_play), video_gap_us: gap(&log.vid_shown), full_restore_us }
}

/// Fresh amplification buckets.
#[must_use]
pub fn amp_buckets() -> Vec<AmpBucket> {
    AMP_EDGES
        .windows(2)
        .map(|w| AmpBucket { frags_lo: w[0], frags_hi: w[1] - 1, ..AmpBucket::default() })
        .chain(std::iter::once(AmpBucket { frags_lo: 4096, frags_hi: 4096, ..AmpBucket::default() }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_percentiles() {
        let v: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&v, 500), Some(50));
        assert_eq!(percentile(&v, 950), Some(95));
        assert_eq!(percentile(&v, 990), Some(99));
        assert_eq!(percentile(&[7], 990), Some(7));
        assert_eq!(percentile(&[], 500), None);
    }

    #[test]
    fn a_freeze_is_a_gap_past_the_webrtc_bar() {
        // 10 fps: step 100 ms, bar max(300, 250) = 300 ms.
        let frames: Vec<VMeta> = (0..40).map(|i| VMeta { capture_us: i * 100_000, key: i == 0, t1: false, frags: 1, bytes: 100 }).collect();
        let mut log = SessionLog { start_us: 0, end_us: 4_000_000, ..SessionLog::default() };
        log.vid_sent = vec![true; 40];
        log.vid_complete = vec![true; 40];
        log.vid_shown = (0..40).map(|i| i * 100_000 + 50_000).collect();
        log.vid_depth = vec![1; 40];
        for i in 10..15 {
            log.vid_shown[i] = 0;
            log.vid_complete[i] = false;
        }
        let rules = QoeRules { fps: 10, capture_encode_us: 0, audio_deadline_us: 400_000, tail_us: 500_000 };
        let mut amp = amp_buckets();
        let q = score_session(&log, &frames, &[], &rules, &mut amp);
        assert_eq!(q.freezes, 1);
        assert_eq!(q.freeze_us, 600_000);
        assert_eq!(q.p0_lost, 5);
        assert_eq!(q.join_us, Some(50_000));
        assert_eq!(q.g2g_median_by_depth[0], Some(50_000));
    }
}
