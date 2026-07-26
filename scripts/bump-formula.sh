#!/bin/sh
# Rewrite Formula/tmtidy.rb for a new release.
# Usage: scripts/bump-formula.sh <version> <sha_arm> <sha_intel>
# Idempotent: re-running with the same inputs produces no change.
set -eu

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <version> <sha_arm> <sha_intel>" >&2
  exit 1
fi

VERSION="$1"
SHA_ARM="$2"
SHA_INTEL="$3"
REPO="chris-str-cst/tmtidy"
BASE="https://github.com/$REPO/releases/download/v$VERSION"
FORMULA="$(dirname "$0")/../Formula/tmtidy.rb"

cat > "$FORMULA" <<EOF
class Tmtidy < Formula
  desc "Keep Time Machine lean by excluding build dirs, with decay-based cleanup"
  homepage "https://github.com/$REPO"
  version "$VERSION"
  license "MIT"

  on_macos do
    on_arm do
      url "$BASE/tmtidy-$VERSION-aarch64-apple-darwin.tar.gz"
      sha256 "$SHA_ARM"
    end
    on_intel do
      url "$BASE/tmtidy-$VERSION-x86_64-apple-darwin.tar.gz"
      sha256 "$SHA_INTEL"
    end
  end

  def install
    bin.install "tmtidy"
  end

  test do
    assert_match "tmtidy", shell_output("#{bin}/tmtidy --version")
  end
end
EOF

echo "wrote $FORMULA (v$VERSION)"
