set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD="$ROOT/build"
TARGET="x86_64-unknown-none"
SECTOR=2048
KERNEL_LIMIT=458752
VM_NAME="NetizenCore"
FALLBACK_WIDTH=1024
FALLBACK_HEIGHT=768

export NC_LOGO_DIR="${NC_LOGO_DIR:-$ROOT/assets/logo}"

fail() {
    echo "hata: $*" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || fail "$1 bulunamadi"
}

file_size() {
    stat -c %s "$1"
}

pick_objcopy() {
    for tool in llvm-objcopy rust-objcopy objcopy; do
        if command -v "$tool" >/dev/null 2>&1; then
            echo "$tool"
            return
        fi
    done
    fail "objcopy bulunamadi"
}

select_iso_tool() {
    if command -v xorriso >/dev/null 2>&1; then
        ISO_CMD=(xorriso -as mkisofs)
    elif command -v genisoimage >/dev/null 2>&1; then
        ISO_CMD=(genisoimage)
    elif command -v mkisofs >/dev/null 2>&1; then
        ISO_CMD=(mkisofs)
    else
        fail "xorriso, genisoimage veya mkisofs gerekli"
    fi
}

check_tools() {
    need nasm
    need cargo
    need stat
    need truncate
}

configure_profile() {
    PROFILE="$1"
    case "$PROFILE" in
        qemu)
            PREFERRED_WIDTH=1280
            PREFERRED_HEIGHT=720
            ;;
        generic)
            PREFERRED_WIDTH=1024
            PREFERRED_HEIGHT=768
            ;;
        *)
            fail "bilinmeyen profil: $PROFILE"
            ;;
    esac
    OUT="$BUILD/$PROFILE"
    ISO="$BUILD/netizencore-$PROFILE.iso"
}

build_bootloader() {
    nasm -f bin "$ROOT/bootloader/stage1.asm" -o "$OUT/stage1.bin"
    nasm -f bin \
        -DPREFERRED_WIDTH="$PREFERRED_WIDTH" \
        -DPREFERRED_HEIGHT="$PREFERRED_HEIGHT" \
        -DFALLBACK_WIDTH="$FALLBACK_WIDTH" \
        -DFALLBACK_HEIGHT="$FALLBACK_HEIGHT" \
        "$ROOT/bootloader/stage2.asm" -o "$OUT/stage2.bin"
    [ "$(file_size "$OUT/stage1.bin")" -eq 2048 ] || fail "stage1 boyutu 2048 olmali"
    [ "$(file_size "$OUT/stage2.bin")" -eq 4096 ] || fail "stage2 boyutu 4096 olmali"
}

build_kernel() {
    if command -v rustup >/dev/null 2>&1; then
        rustup target add "$TARGET"
    fi
    RUSTFLAGS="-C relocation-model=static" \
        cargo build --release \
        --manifest-path "$ROOT/kernel/Cargo.toml" \
        --target "$TARGET" \
        --target-dir "$BUILD/cargo-$PROFILE" \
        --features "$PROFILE"
    "$(pick_objcopy)" -O binary "$BUILD/cargo-$PROFILE/$TARGET/release/kernel" "$OUT/kernel.bin"
    local size
    size="$(file_size "$OUT/kernel.bin")"
    [ "$size" -gt 0 ] || fail "kernel.bin bos"
    [ "$size" -le "$KERNEL_LIMIT" ] || fail "kernel $KERNEL_LIMIT bayttan buyuk olamaz (simdiki: $size)"
    local padded=$(( (size + SECTOR - 1) / SECTOR * SECTOR ))
    truncate -s "$padded" "$OUT/kernel.bin"
    echo "kernel boyutu: $size bayt"
}

build_iso() {
    select_iso_tool
    mkdir -p "$OUT/iso/boot"
    cat "$OUT/stage1.bin" "$OUT/stage2.bin" "$OUT/kernel.bin" > "$OUT/iso/boot/ncboot.bin"
    "${ISO_CMD[@]}" \
        -V NETIZENCORE \
        -b boot/ncboot.bin \
        -c boot/boot.cat \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -o "$ISO" \
        "$OUT/iso"
}

build_profile() {
    configure_profile "$1"
    rm -rf "$OUT"
    mkdir -p "$OUT"
    build_bootloader
    build_kernel
    build_iso
    echo "hazir: $ISO"
}

run_qemu() {
    need qemu-system-x86_64
    local accel=()
    if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        accel=(-enable-kvm -cpu host)
    fi
    qemu-system-x86_64 \
        ${accel[@]+"${accel[@]}"} \
        -m 256 \
        -cdrom "$ISO" \
        -boot d \
        -vga std \
        -serial stdio \
        -debugcon "file:$BUILD/debugcon.log" \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
        -no-reboot
}

run_vbox() {
    need VBoxManage
    if ! VBoxManage showvminfo "$VM_NAME" >/dev/null 2>&1; then
        VBoxManage createvm --name "$VM_NAME" --ostype Other_64 --register
        VBoxManage modifyvm "$VM_NAME" --memory 256 --vram 16 --firmware bios \
            --boot1 dvd --boot2 none --boot3 none --boot4 none
        VBoxManage storagectl "$VM_NAME" --name IDE --add ide
    fi
    VBoxManage storageattach "$VM_NAME" --storagectl IDE --port 0 --device 0 \
        --type dvddrive --medium "$ISO"
    VBoxManage startvm "$VM_NAME"
}

cd "$ROOT"

case "${1:-qemu}" in
    qemu)
        check_tools
        build_profile qemu
        ;;
    generic)
        check_tools
        build_profile generic
        ;;
    all)
        check_tools
        build_profile qemu
        build_profile generic
        ;;
    run)
        check_tools
        build_profile qemu
        run_qemu
        ;;
    vbox)
        check_tools
        build_profile generic
        run_vbox
        ;;
    clean)
        rm -rf "$BUILD"
        ;;
    *)
        echo "kullanim: bash build.sh [qemu|generic|all|run|vbox|clean]" >&2
        exit 1
        ;;
esac
