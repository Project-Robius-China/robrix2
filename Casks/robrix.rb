cask "robrix" do
  arch arm: "aarch64", intel: "x86_64"

  version "1.1.0"
  sha256 arm:   "783375cd75c8fc3ad38ccc3ad70e8368faf83606ad4788b99118cf6af6404e7a",
         intel: "cbfb903acbf3ba68cb1549878e0847b5d6d53318fec7d7bc1523808cc63889b2"

  url "https://github.com/Project-Robius-China/robrix2/releases/download/v1.1.0/robrix-#{version}-macos-#{arch}-release.dmg"
  name "Robrix"
  desc "Multi-platform Matrix chat client built with Rust and Makepad"
  homepage "https://github.com/Project-Robius-China/robrix2"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: :big_sur

  app "Robrix.app"
end
