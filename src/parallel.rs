use std::thread;

const DEFAULT_PARALLEL_FILE_THRESHOLD: usize = 3;

pub fn worker_count(paths: &[String], jobs: Option<usize>) -> Option<usize> {
    if paths.len() <= 1 || paths.iter().any(|path| path == "-") || jobs == Some(1) {
        return None;
    }

    if jobs.is_none() && paths.len() <= DEFAULT_PARALLEL_FILE_THRESHOLD {
        return None;
    }

    Some(
        jobs.or_else(|| thread::available_parallelism().map(usize::from).ok())
            .unwrap_or(1)
            .min(paths.len()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_serial_until_parallel_file_threshold() {
        let paths = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];

        assert_eq!(worker_count(&paths, None), None);
    }

    #[test]
    fn defaults_to_parallel_after_parallel_file_threshold() {
        let paths = vec![
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "d".to_owned(),
        ];

        assert!(worker_count(&paths, None).is_some());
    }

    #[test]
    fn explicit_jobs_overrides_parallel_file_threshold() {
        let paths = vec!["a".to_owned(), "b".to_owned()];

        assert_eq!(worker_count(&paths, Some(2)), Some(2));
    }
}
