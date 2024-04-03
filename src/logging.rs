/// creates a log on the trace level
macro_rules! trace {
	($($expr:expr),*) => (
		#[cfg(feature = "tracing")]
		{
			tracing::trace!($($expr),*);
		}

		#[cfg(feature = "log")]
		{
			log::trace!($($expr),*);
		}

		#[cfg(not(any(feature = "tracing", feature = "log")))]
		{
			log_allow_unused!($($expr),*);
		}
	)
}

/// creates a log on the debug level
macro_rules! debug {
	($($expr:expr),*) => (
		#[cfg(feature = "tracing")]
		{
			tracing::debug!($($expr),*);
		}

		#[cfg(feature = "log")]
		{
			log::debug!($($expr),*);
		}

		#[cfg(not(any(feature = "tracing", feature = "log")))]
		{
			log_allow_unused!($($expr),*);
		}
	)
}

/// allow expressions to be unused
#[allow(unused)]
macro_rules! log_allow_unused {
	($($expr:expr),*) => (
		$(
			let _ = $expr;
		)*
	)
}
