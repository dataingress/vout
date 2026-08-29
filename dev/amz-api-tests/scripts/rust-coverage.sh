#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PROJECT_DIR}/../.." && pwd)"
PROFILE_DIR="${REPO_ROOT}/target/coverage/amz-api-tests"
TARGET_DIR="${REPO_ROOT}/target/coverage-target"
BINARY="${TARGET_DIR}/debug/vout"
PROFDATA="${PROFILE_DIR}/vout.profdata"
REPORT="${PROFILE_DIR}/amz-api-coverage.txt"

find_llvm_tool() {
  local tool="$1"

  if command -v "${tool}" >/dev/null 2>&1; then
    command -v "${tool}"
    return 0
  fi

  local host
  local sysroot
  host="$(rustc -vV | awk '/host:/ { print $2 }')"
  sysroot="$(rustc --print sysroot 2>/dev/null || true)"
  if [[ -n "${sysroot}" && -x "${sysroot}/lib/rustlib/${host}/bin/${tool}" ]]; then
    printf '%s\n' "${sysroot}/lib/rustlib/${host}/bin/${tool}"
    return 0
  fi

  for candidate in /usr/lib/llvm-*/bin/"${tool}" /usr/lib64/llvm-*/bin/"${tool}"; do
    if [[ -x "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

LLVM_PROFDATA="$(find_llvm_tool llvm-profdata || true)"
LLVM_COV="$(find_llvm_tool llvm-cov || true)"

if [[ -z "${LLVM_PROFDATA}" || -z "${LLVM_COV}" ]]; then
  cat >&2 <<'EOF'
Missing LLVM coverage tools: llvm-profdata and/or llvm-cov.

Install a Rust toolchain with llvm-tools-preview or install the system LLVM tools,
then rerun `bun run rust:coverage`.
EOF
  exit 1
fi

rm -rf "${PROFILE_DIR}"
mkdir -p "${PROFILE_DIR}"

export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="${PROFILE_DIR}/vout-%p-%m.profraw"
export VOUT_KEYRING_KEY_VERSION="amz_api_tests"

cargo run -p clear-keyring --target-dir "${TARGET_DIR}" --manifest-path "${REPO_ROOT}/Cargo.toml"
cargo build -p vout --target-dir "${TARGET_DIR}" --manifest-path "${REPO_ROOT}/Cargo.toml"

(
  cd "${PROJECT_DIR}"
  VOUT_TEST_BINARY="${BINARY}" bun test
)

"${LLVM_PROFDATA}" merge -sparse "${PROFILE_DIR}"/*.profraw -o "${PROFDATA}"
"${LLVM_COV}" report \
  "${BINARY}" \
  --instr-profile="${PROFDATA}" \
  --ignore-filename-regex='/.cargo/registry|/rustc/' \
  "${REPO_ROOT}/source/vout/src/server/amz/"*.rs \
  | tee "${REPORT}"

awk '
  $1 ~ /^(delete_parameter|delete_parameters|describe_parameters|get_parameter_history|get_parameter|get_parameters_by_path|get_parameters|label_parameter_version|mod|put_parameter|unlabel_parameter_version)\.rs$/ {
    files++;
    if ($4 != "100.00%" || $7 != "100.00%" || $10 != "100.00%") {
      failed = 1;
    }
  }
  END {
    if (files == 0) {
      print "No AMZ API files found in coverage report." > "/dev/stderr";
      exit 1;
    }
    if (failed) {
      print "AMZ API coverage is below 100%." > "/dev/stderr";
      exit 1;
    }
  }
' "${REPORT}"
