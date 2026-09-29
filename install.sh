#!/usr/bin/env bash

set -e
set -o pipefail

# Install summon-keepass. Default is the latest version. Pass a valid version tag to install a dedicated version.

error() {
  echo "ERROR: $@" 1>&2
  echo "Exiting installer" 1>&2
  exit 1
}

VERSION=${1:-"latest"}

PROJECT="summon-keepass"
REPO_NAME="desolat/summon-keepass"

ARCH=`uname -m`
case "$ARCH" in
  x86_64) ARCH_SUFFIX="amd64" ;;
  aarch64|arm64) ARCH_SUFFIX="arm64" ;;
  *) error "$PROJECT is not available for architecture: ${ARCH}" ;;
esac

KERNEL_NAME=`uname | tr "[:upper:]" "[:lower:]"`

if [ "${KERNEL_NAME}" != "linux" ]; then # && [ "${KERNEL_NAME}" != "darwin" ]
  error "This installer currently only supports Linux"
fi

tmp="/tmp"
if [ ! -z "$TMPDIR" ]; then
  tmp=$TMPDIR
fi

# secure-ish temp dir creation without having mktemp available (DDoS-able but not exploitable)
tmp_dir="$tmp/install.sh.$$"
(umask 077 && mkdir $tmp_dir) || exit 1

# do_download URL DIR
do_download() {
  echo "Downloading $1"
  if [[ $(command -v wget) ]]; then
    wget -q -O "$2" "$1" >/dev/null
  elif [[ $(command -v curl) ]]; then
    curl --fail -sSL -o "$2" "$1" &>/dev/null || true
  else
    error "Could not find wget or curl"
  fi
}

# Get latest release from GitHub API
get_latest_version() {
  local LATEST_VERSION_URL="https://api.github.com/repos/$REPO_NAME/releases/latest"
  local latest_payload

  if [[ $(command -v wget) ]]; then
    latest_payload=$(wget -q -O - "$LATEST_VERSION_URL")
  elif [[ $(command -v curl) ]]; then
    latest_payload=$(curl --fail -sSL "$LATEST_VERSION_URL")
  else
    error "Could not find wget or curl"
  fi

  echo "$latest_payload" |
    grep '"tag_name":' | # Get tag line
    sed -E 's/.*"([^"]+)".*/\1/' # Pluck JSON value
}

# Check if a release for this version exists
check_version() {
  local VERSION=$1
  local RELEASE_URL="https://api.github.com/repos/$REPO_NAME/releases/tags/$VERSION"

  if [[ $(command -v wget) ]]; then
    wget -q -O /dev/null "$RELEASE_URL"
  elif [[ $(command -v curl) ]]; then
    curl --fail -sSL "$RELEASE_URL" >/dev/null
  else
    error "Could not find wget or curl"
  fi
}

if [ $VERSION == "latest" ]; then
  VERSION=$(get_latest_version)
  echo "Latest version: $VERSION"
else
  check_version $VERSION
fi

FILE_NAME="$PROJECT-${KERNEL_NAME}-${ARCH_SUFFIX}.tar.gz"
URL="https://github.com/$REPO_NAME/releases/download/${VERSION}/$FILE_NAME"

FILE_PATH="${tmp_dir}/$FILE_NAME"
do_download ${URL} ${FILE_PATH}

TARGET_PATH="/usr/local/bin"
echo "Installing $PROJECT ${VERSION} into $TARGET_PATH"

if [[ "$FILE_PATH" == *.tar.gz ]]; then
  if sudo -h >/dev/null 2>&1; then
    sudo tar -C $TARGET_PATH -o -zxvf ${FILE_PATH} >/dev/null
  else
    tar -C $TARGET_PATH -o -zxvf ${FILE_PATH} >/dev/null
  fi
else
  cp $FILE_PATH "${TARGET_PATH}"
fi

echo "Installed $PROJECT to $TARGET_PATH"
