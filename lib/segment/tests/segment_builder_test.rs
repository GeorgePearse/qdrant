mod fixtures;

#[cfg(test)]
mod tests {
    use crate::fixtures::segment::{build_segment_1, build_segment_2, empty_segment};
    use itertools::Itertools;
    use segment::entry::entry_point::{OperationError, SegmentEntry};
    use segment::segment::Segment;
    use segment::segment_constructor::segment_builder::SegmentBuilder;
    use segment::types::{Indexes, SegmentConfig};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tempdir::TempDir;

    #[test]
    fn test_building_new_segment() {
        let dir = TempDir::new("segment_dir").unwrap();
        let temp_dir = TempDir::new("segment_temp_dir").unwrap();

        let stopped = AtomicBool::new(false);

        // let segment1_dir = dir.path().join("segment_1");
        // let segment2_dir = dir.path().join("segment_2");

        let segment1 = build_segment_1(dir.path());
        let mut segment2 = build_segment_2(dir.path());

        let mut builder =
            SegmentBuilder::new(dir.path(), temp_dir.path(), &segment1.segment_config).unwrap();

        // Include overlapping with segment1 to check the
        segment2
            .upsert_point(100, 3.into(), &[0., 0., 0., 0.])
            .unwrap();

        builder.update_from(&segment1, &stopped).unwrap();
        builder.update_from(&segment2, &stopped).unwrap();
        builder.update_from(&segment2, &stopped).unwrap();

        // Check what happens if segment building fails here

        let segment_count = dir.path().read_dir().unwrap().count();

        assert_eq!(segment_count, 2);

        let temp_segment_count = temp_dir.path().read_dir().unwrap().count();

        assert_eq!(temp_segment_count, 1);

        // Now we finalize building

        let merged_segment: Segment = builder.build(&stopped).unwrap();

        let new_segment_count = dir.path().read_dir().unwrap().count();

        assert_eq!(new_segment_count, 3);

        assert_eq!(
            merged_segment.vectors_count(),
            segment1
                .iter_points()
                .chain(segment2.iter_points())
                .unique()
                .count()
        );

        assert_eq!(merged_segment.point_version(3.into()), Some(100));
    }

    fn estimate_build_time(segment: &Segment, stop_timeout_millis: u64) -> (u64, bool) {
        let stopped = Arc::new(AtomicBool::new(false));

        let dir = TempDir::new("segment_dir1").unwrap();
        let temp_dir = TempDir::new("segment_temp_dir").unwrap();

        let segment_config = SegmentConfig {
            vector_size: segment.segment_config.vector_size,
            distance: segment.segment_config.distance,
            index: Indexes::Hnsw(Default::default()),
            storage_type: Default::default(),
            payload_storage_type: Default::default(),
        };

        let mut builder =
            SegmentBuilder::new(dir.path(), temp_dir.path(), &segment_config).unwrap();

        builder.update_from(segment, &*stopped).unwrap();

        let now = Instant::now();

        let stopped_t = stopped.clone();

        std::thread::Builder::new()
            .name("build_estimator_timeout".to_string())
            .spawn(move || {
                std::thread::sleep(Duration::from_millis(stop_timeout_millis));
                stopped_t.store(true, Ordering::Release);
            })
            .unwrap();

        let res = builder.build(&*stopped);

        let is_cancelled = match res {
            Ok(_) => false,
            Err(err) => matches!(err, OperationError::Cancelled { .. }),
        };

        (now.elapsed().as_millis() as u64, is_cancelled)
    }

    #[test]
    fn test_building_cancellation() {
        let dir = TempDir::new("segment_dir").unwrap();

        let mut segment = empty_segment(dir.path());

        for idx in 0..1000 {
            segment
                .upsert_point(1, idx.into(), &[0., 0., 0., 0.])
                .unwrap();
        }

        // Checks that optimization with longed cancellation timeout will also finishes fast
        let (time_fast, is_stopped_fast) = estimate_build_time(&segment, 20);
        let (time_long, _is_stopped_long) = estimate_build_time(&segment, 200);

        assert!(is_stopped_fast);

        assert!(time_fast < time_long);
    }
}
