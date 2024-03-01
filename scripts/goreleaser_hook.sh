#!/usr/bin/env bash

set -eou pipefail
set -x

go_arch=$1
go_os=$2
project_name=$3

# Make Go -> Rust arch/os mapping
case $go_arch in
    x86_64) rust_arch='x86_64' ;;
    amd64) rust_arch='x86_64' ;;
    aarch64) rust_arch='aarch64' ;;
    arm64) rust_arch='aarch64' ;;
    *) echo "unknown arch: $go_arch" && exit 1 ;;
esac
case $go_os in
    linux) rust_os='linux' ;;
    darwin) rust_os='darwin' ;;
    *) echo "unknown os: $go_os" && exit 1 ;;
esac

echo "Before copying"
ls -lah dist/*
ls -lah artifacts/*

# Find artifacts and uncompress in the corresponding directory
find . -type f -name "*${rust_arch}*${rust_os}*" -exec unzip -d dist/${project_name}_${go_os}_${go_arch} {} \;

echo "After copying"
ls -lah dist/*
ls -lah artifacts/*
