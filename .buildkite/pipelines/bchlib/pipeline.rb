# frozen_string_literal: true

Buildkite::Builder.pipeline do
  use(NixExt)

  group do
    label "bchlib", emoji: :crab

    nix("rust_stable", ".", "cargo fmt --all -- --check", label: "cargo fmt --check")

    # bchlib-sys/bchlib expose three independent Cargo feature axes (see
    # bchlib-sys/Cargo.toml): `std` (no_std attribute + bindgen core/std
    # toggle), `malloc` (C-side allocator: real malloc() vs the fixed
    # 24576-byte static heap), and `decode` (whether decode_bch/correct_bch
    # are compiled in at all - see bch.c's #ifdef BCH_DECODE gating). These
    # five combinations exercise every meaningfully distinct code path:
    #   default                  - std + malloc + decode: hosted build, what
    #                              most consumers use.
    #   static heap, encode-only - std still ON, but malloc/decode OFF: lets
    #                              the static-heap allocator run under a
    #                              normal test harness for convenience.
    #   static heap, decode      - same static heap, decode compiled in too.
    #   no_std, encode-only      - std genuinely OFF (#![no_std] active) on
    #                              top of the static heap. This is the actual
    #                              embedded/flight-firmware target shape; it
    #                              was never exercised anywhere before this
    #                              (every "no_std-style" config above still
    #                              had std on) - #![no_std] only forbids
    #                              referencing std, it doesn't require a
    #                              std-less target to catch a violation, so
    #                              this is a real, meaningful check even on
    #                              a normal host target.
    #   no_std, decode           - same, with decode compiled in.
    configs = [
      { name: "default", features: nil },
      { name: "static heap, encode-only", features: ["std"] },
      { name: "static heap, decode", features: %w[std decode] },
      { name: "no_std, encode-only", features: [] },
      { name: "no_std, decode", features: ["decode"] },
    ].map do |config|
      enabled = config[:features] || %w[std malloc decode] # nil means "cargo defaults"
      flags =
        if config[:features].nil?
          ""
        elsif config[:features].empty?
          "--no-default-features"
        else
          "--no-default-features --features #{config[:features].join(',')}"
        end
      config.merge(enabled: enabled, flags: flags)
    end

    configs.each do |config|
      nix(
        "rust_stable", ".",
        "cargo clippy --workspace #{config[:flags]} --locked -- -D warnings",
        label: "cargo clippy: #{config[:name]}"
      )
    end

    configs.each do |config|
      # The static-heap allocator (used whenever `malloc` is off, i.e. every
      # non-default config above) shares one un-synchronized global buffer
      # across every init_bch() call. Rust's default test harness runs tests
      # concurrently, which races on that shared state and can crash - not a
      # bug in these particular tests, but a real gap in the allocator's
      # thread-safety that hasn't been fixed yet. Force serial execution for
      # any config that isn't `malloc`-backed until that's addressed.
      needs_serial = !config[:enabled].include?("malloc")
      test_cmd = "cargo test --workspace #{config[:flags]}"
      test_cmd += " -- --test-threads=1" if needs_serial

      nix(
        "rust_stable", ".",
        test_cmd,
        label: "cargo test: #{config[:name]}"
      )

      # exhaustive_m_t_sweep (bchlib/src/lib.rs) is #[ignore]d by default and
      # gated behind std+malloc+decode: it sweeps the entire valid (m, t)
      # parameter space (~4700 pairs), some of which build multi-megabyte
      # tables that need a real, unconstrained allocator, and it takes
      # several minutes even in --release. Run it for whichever configs
      # actually satisfy that feature combination, instead of only ever
      # running it by hand - currently that's just "default", but this
      # stays correct if that ever changes.
      needs_exhaustive = %w[std malloc decode].all? { |f| config[:enabled].include?(f) }
      next unless needs_exhaustive

      nix(
        "rust_stable", ".",
        "cargo test --release --workspace #{config[:flags]} -- --ignored exhaustive_m_t_sweep",
        label: "cargo test --release: exhaustive_m_t_sweep (#{config[:name]})",
        timeout_mins: 15
      )
    end
  end
end
