# `radio` module

This module implements a low level control layer for different radio devices.
For example, an RTL-SDR/USRP attached to the host. Or a remote SDR streaming I/Q samples over TCP/UDP (raw, or using VITA-49).

Its task is to send/receive samples from the hardware device, do some light processing (maybe BPF, FFT), and stream them to another destination.
The destination may be the Web UI, a software modem, a file recording, etc.

May make sense to start with an implementation using SoapySDR, as that probably covers most relevant SDRs.
