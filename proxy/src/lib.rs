use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(feature = "use_old", feature = "use_new"))]
    {
        compile_error!("Can't use 'use_old' AND 'use_new' together");
    }
    else if #[cfg(all(not(feature = "use_old"), not(feature = "use_new")))]
    {
        compile_error!("Must use either 'use_old' OR 'use_new' feature flag, you must choose one!");
    }
}

cfg_if! {
    if #[cfg(feature = "use_old")]
    {
        pub use webrtc_unreliable_client_old::*;
    }
}

cfg_if! {
    if #[cfg(feature = "use_new")]
    {
        pub use webrtc_unreliable_client_new::*;
    }
}