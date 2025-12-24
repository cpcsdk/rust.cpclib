/// Macro for hash-based, case-insensitive directive matching with optional guard
#[macro_export]
macro_rules! hashed_choice {
    // With guard: delegate to the main branch, then add the guard
    ($hash:expr, $word:expr, $($lit:expr),+,  if $guard:expr) => {
        ($guard) && hashed_choice!($hash, $word, $($lit),+)
    };
    // Main branch: no guard
    ($hash:expr, $word:expr, $($lit:expr),+) => {
        (
            $(
                ($hash == fnv1a_ascii_upper($lit) && eq_ascii_nocase($word, $lit)) ||
            )+
            false
        )
    };
}
