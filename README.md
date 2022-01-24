# remotia

## Introduction

remotia is an open source framework for building remote rendering software, implemented in pure Rust.

The objective of this project is to provide:

- A easy to customize benchmarking tool to researchers that want to evaluate their network communication protocols and encoders in a context of real-time video streaming.

- A solid basis for teams that want to develop both the server and the client part of a remote rendering solution, with a special focus on cloud gaming and real-time desktop streaming.

## Author's note

As of the time of writing, remotia is a one-man PhD project. My aim is to fill the gap that other discontinued projects like GamingAnywhere left, providing an OS/Hardware agnostic platform to run the same kind of experiments while relying on a modern language like Rust.
Contributions, tips and reviews are appreciated and encouraged.

Lorenzo.

## Usage

You may find some examples including different architectures, codecs and communication protocols. Apart the base architecture (silo...) most of the other components are configurable at runtime via the command line. Use the --help option to know more about each example.