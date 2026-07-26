class Tmtidy < Formula
  desc "Keep Time Machine lean by excluding build dirs, with decay-based cleanup"
  homepage "https://github.com/chris-str-cst/tmtidy"
  version "0.1.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/chris-str-cst/tmtidy/releases/download/v0.1.1/tmtidy-0.1.1-aarch64-apple-darwin.tar.gz"
      sha256 "130fc17a617d2446e66d114eb0a45ac02dd041874de74b0cbe2716e82fc77d46"
    end
    on_intel do
      url "https://github.com/chris-str-cst/tmtidy/releases/download/v0.1.1/tmtidy-0.1.1-x86_64-apple-darwin.tar.gz"
      sha256 "8e3b92ac3c36a14eca9898062299c8dc8b7afc94728b43034846254172734909"
    end
  end

  def install
    bin.install "tmtidy"
  end

  test do
    assert_match "tmtidy", shell_output("#{bin}/tmtidy --version")
  end
end
