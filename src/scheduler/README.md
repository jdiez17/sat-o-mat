# `scheduler` Module

This module is responsible for coordinating activities involving the other modules.
It is conceptually simple. 
It maintains a "schedule", which is a list of "programs" (feel free to suggest a better name).
Each program is a list of submodule commands that shall be executed at a predefined time, or immediately.

Schedule requests are received via the API.
According to configuration, requests are either auto-accepted, evaluated by a script, or require evaluation by an administrator.

## API

The scheduling service should be exposed using an API.
To be determined if this should be a web-like REST API or gRPC / Capn' proto or similar.
It needs to handle authentication; not all users can access all API endpoints.


