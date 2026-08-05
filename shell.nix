{ pkgs ? import <nixpkgs> {} }:

let
  # Wir holen uns die OpenSSL-Pakete direkt aus der Musl-Target-Toolchain
  muslPkgs = pkgs.pkgsCross.musl64;
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustup
    pkg-config
    # Die Cross-Compilation Toolchain für musl
    pkgsCross.musl64.stdenv.cc
  ];

  # Zusätzliche Build-Abhängigkeiten (Libraries für das Target)
  buildInputs = [
    muslPkgs.openssl.dev
  ];

  shellHook = ''
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="x86_64-unknown-linux-musl-gcc"
    
    # Compiler-Flags für C-Abhängigkeiten
    export CC_x86_64_unknown_linux_musl="x86_64-unknown-linux-musl-gcc"
    export CXX_x86_64_unknown_linux_musl="x86_64-unknown-linux-musl-g++"
    
    # OpenSSL-Pfade explizit für das Musl-Target verbiegen
    export X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR="${muslPkgs.openssl.dev}"
    export X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_LIB_DIR="${muslPkgs.openssl.out}/lib"
    export X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_INCLUDE_DIR="${muslPkgs.openssl.dev}/include"
    
    # Erzwinge das statische Linken von OpenSSL für das Target
    export OPENSSL_STATIC=1
    
    echo "🦀 Rust Cross-Shell (musl + static OpenSSL) geladen!"
    echo "Nutze: cargo build --release --target x86_64-unknown-linux-musl"
  '';
}
