# Sat-O-Mat

An application to control satellite ground station hardware.

It consists of the following modules:

- [Web UI](...) [insert screenshots here]
- [`tracker`](...)
   - Calculates the trajectory of a satellite over a ground station
   - Calculates (azimuth, elevation) angles and controls a [Hamlib](...)-compatible rotator
   - Calculates Doppler corrections for the satellite's up/downlink frequencies and sends them to a [Hamlib](...)-compatible radio transceiver or an SDR-based modem.
- [`radio`](...) opens a connection to one or more [Software Defined Radio](...) devices and acts as a TX/RX bridge
    - Sends/receives IQ samples to/from TCP/UDP/[ZeroMQ](...) to software modems
        - See [`akira25/gnuradio-docker-container`](https://github.com/akira25/gnuradio-docker-container) for an example flowgraph and further explanation of the motivation for this
    - Computes FFT and other radio diagnostics and sends to Web UI
    - Provides an optional Doppler-corrected IQ stream
    - Optionally records corrected or uncorrected streams into files
- [`executor`](...) runs and monitors various subprograms (scripts, GNU Radio flowcharts, [Linux Containers](...) etc.) and monitors them.
- [`scheduler`](...) receives pass schedule requests from many sources (Web UI, API, SatNOGS, ...)
    - Requests can be auto-approved by default, evaluated by a script, or manually evaluated in the Web UI
