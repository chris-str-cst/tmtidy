class Tmtidy < Formula
  desc "Keep Time Machine lean by excluding build dirs, with decay-based cleanup"
  homepage "https://github.com/chris-str-cst/tmtidy"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/chris-str-cst/tmtidy/releases/download/v0.1.0/tmtidy-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "fbdd7de34f3399820c69f62696a0111cc7b6946d8d9ada71f7cd669f6926535f"
    end
    on_intel do
      url "https://github.com/chris-str-cst/tmtidy/releases/download/v0.1.0/tmtidy-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "22fa19cc5ad271a036ff1b876f8d2985439f0fdad14379c62d4920775cefdb4a"
    end
  end

  def install
    bin.install "tmtidy"
  end

  test do
    assert_match "tmtidy", shell_output("#{bin}/tmtidy --version")
  end
end
