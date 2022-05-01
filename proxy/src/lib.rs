use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(feature = "use_webrtc", feature = "use_proxy"))]
    {
        compile_error!("Can't use 'use_webrtc' AND 'use_proxy' together");
    }
    else if #[cfg(all(not(feature = "use_webrtc"), not(feature = "use_proxy")))]
    {
        compile_error!("Must use either 'use_webrtc' OR 'use_proxy' feature flag, you must choose one!");
    }
}

cfg_if! {
    if #[cfg(feature = "use_webrtc")]
    {
        pub use webrtc::*;
    }
}

cfg_if! {
    if #[cfg(feature = "use_proxy")]
    {
        pub use webrtc_unreliable_client_slim::*;
    }
}