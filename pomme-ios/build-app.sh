#!/usr/bin/env bash
# Xcode's build step: builds the client for the platform being built, with
# MoltenVK linked in, as the app's executable, and embeds OpenAL Soft, which
# the client loads at runtime. Xcode signs the app after this.
set -euo pipefail

case "$PLATFORM_NAME" in
    iphonesimulator) triple=aarch64-apple-ios-sim platform=ios-simulator ;;
    iphoneos) triple=aarch64-apple-ios platform=ios ;;
    *) echo "error: unsupported platform $PLATFORM_NAME" >&2; exit 1 ;;
esac
profile=debug
release=
if [ "$CONFIGURATION" = Release ]; then
    profile=release
    release=--release
fi

app="$TARGET_BUILD_DIR/$WRAPPER_NAME"
frameworks="$TARGET_BUILD_DIR/$FRAMEWORKS_FOLDER_PATH"
export PATH="$HOME/.cargo/bin:$PATH"

# Xcode sets SDKROOT to the iPhoneOS SDK. Cargo also builds host-side native
# dependencies (for example aws-lc-sys), which need the macOS SDK instead.
# Keep the host SDK as the default, and explicitly select the iPhoneOS SDK for
# C dependencies compiled for the aarch64-apple-ios target.
HOST_MACOS_SDK="$(xcrun --sdk macosx --show-sdk-path)"
IOS_SDK="$(xcrun --sdk iphoneos --show-sdk-path)"
export SDKROOT="$HOST_MACOS_SDK"
export CC_aarch64_apple_ios="$(xcrun --sdk iphoneos --find clang)"
export CFLAGS_aarch64_apple_ios="-isysroot $IOS_SDK -miphoneos-version-min=16.0"
unset LIBRARY_PATH

cd "$PROJECT_DIR/.."
# Keep the host Vulkan SDK available for shaderc-sys. The iOS link step needs
# MoltenVK's libvulkan.a, so stage MoltenVK separately and expose the host SDK's
# Vulkan registry under that directory (shaderc-sys reads share/vulkan/registry/vk.xml).
HOST_VULKAN_SDK="${VULKAN_SDK:-}"
if [ -n "$HOST_VULKAN_SDK" ]; then
    export SHADERC_LIB_DIR="$HOST_VULKAN_SDK/lib"
fi
MOLTENVK_SDK="$(python3 tools/fetch_moltenvk.py "$platform")"
if [ -n "$HOST_VULKAN_SDK" ] && [ -d "$HOST_VULKAN_SDK/share/vulkan" ]; then
    mkdir -p "$MOLTENVK_SDK/share"
    ln -sfn "$HOST_VULKAN_SDK/share/vulkan" "$MOLTENVK_SDK/share/vulkan"
fi
export VULKAN_SDK="$MOLTENVK_SDK"

cargo build -p pomme-ios --target "$triple" ${release:+"$release"}
mkdir -p "$app" "$frameworks"
# Copied only when it changed, so an unchanged build isn't re-signed.
if ! cmp -s "target/$triple/$profile/pomme-ios" "$app/$EXECUTABLE_NAME"; then
    cp "target/$triple/$profile/pomme-ios" "$app/$EXECUTABLE_NAME"
fi

python3 tools/build_openal.py "$platform" "$frameworks"
if [ -n "${EXPANDED_CODE_SIGN_IDENTITY:-}" ]; then
    codesign --force --sign "$EXPANDED_CODE_SIGN_IDENTITY" --timestamp=none \
        "$frameworks/soft_oal.framework"
fi
