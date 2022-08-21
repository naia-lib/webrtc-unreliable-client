# webrtc-unreliable-client
![](https://tokei.rs/b1/github/naia-lib/naia-unreliable-client)
[![MIT/Apache][s3]][l3]

[s3]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[l3]: docs/LICENSE-MIT

Just enough hacks to connect a native client to a [webrtc-unreliable](https://github.com/triplehex/webrtc-unreliable) server

Currently the codebase is slimmed down quite a bit from it's [webrtc-rs](https://github.com/webrtc-rs/webrtc)
roots, however, many more optimizations can be done in order to optimize
compile-time and code size. There is so much code here that will never be ran in the
scenario it is meant to be used.

If anyone knows of a good way to anayze during run-time which functions are never called, please get in touch!

