#!/usr/bin/env bash

# WARNING: only works on Github Actions
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
ls -Rlah ./*

# ./artifacts/tome-darwin-x86_64.tar.gz
# /home/runner/work/tome/tome/artifacts/tome-darwin-aarch64.tar.gz
# Find artifacts and uncompress in the corresponding directory

mv "./artifacts/${project_name}-${rust_os}-${rust_arch}/${project_name}" "dist/${project_name}_${go_os}_${go_arch}"

echo "After copying"
ls -lah dist/*
ls -Rlah ./*
