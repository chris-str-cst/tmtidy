class Tmtidy < Formula
  desc "Keep Time Machine lean by excluding build dirs, with decay-based cleanup"
  homepage "https://github.com/chris-str-cst/tmtidy"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/chris-str-cst/tmtidy/releases/download/v0.1.0/tmtidy-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM_SHA256_REPLACED_ON_FIRST_RELEASE"
    end
    on_intel do
      url "https://github.com/chris-str-cst/tmtidy/releases/download/v0.1.0/tmtidy-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_INTEL_SHA256_REPLACED_ON_FIRST_RELEASE"
    end
  end

  def install
    bin.install "tmtidy"
  end

  test do
    assert_match "tmtidy", shell_output("#{bin}/tmtidy --version")
  end
end
