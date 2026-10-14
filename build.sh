set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD="$ROOT/build"
TARGET="x86_64-unknown-none"
UEFI_TARGET="x86_64-unknown-uefi"
SECTOR=2048
KERNEL_LIMIT=16777216
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
    nasm -f bin "$ROOT/bootloader/mbr.asm" -o "$ROOT/crates/nc-storage/mbr.bin"
    [ "$(file_size "$ROOT/crates/nc-storage/mbr.bin")" -eq 446 ] || fail "mbr boyutu 446 olmali"

    nasm -f bin "$ROOT/bootloader/stage1.asm" -o "$OUT/stage1.bin"
    [ "$(file_size "$OUT/stage1.bin")" -eq 2048 ] || fail "stage1 boyutu 2048 olmali"

    nasm -f bin \
        -DPREFERRED_WIDTH="$PREFERRED_WIDTH" \
        -DPREFERRED_HEIGHT="$PREFERRED_HEIGHT" \
        -DFALLBACK_WIDTH="$FALLBACK_WIDTH" \
        -DFALLBACK_HEIGHT="$FALLBACK_HEIGHT" \
        "$ROOT/bootloader/stage2.asm" -o "$OUT/stage2.bin"
    [ "$(file_size "$OUT/stage2.bin")" -eq 4096 ] || fail "stage2 boyutu 4096 olmali"
}

build_kernel() {
    if command -v rustup >/dev/null 2>&1; then
        rustup target add "$TARGET"
    fi
    RUSTFLAGS='-C relocation-model=static --cfg chacha20_force_soft --cfg poly1305_force_soft --cfg curve25519_dalek_backend="serial"' \
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
    echo "kernel boyutu: $size bayt (dinamik yukleyici aktif)"
}

build_uefi_optional() {
    if [ -f "$ROOT/crates/nc-uefi/Cargo.toml" ] && command -v rustup >/dev/null 2>&1; then
        rustup target add "$UEFI_TARGET" >/dev/null 2>&1 || true
        NC_KERNEL_BIN="$OUT/kernel.bin" cargo build --release \
            --manifest-path "$ROOT/crates/nc-uefi/Cargo.toml" \
            --target "$UEFI_TARGET" \
            --target-dir "$BUILD/cargo-uefi-$PROFILE" && \
        mkdir -p "$OUT/iso/EFI/BOOT" && \
        cp "$BUILD/cargo-uefi-$PROFILE/$UEFI_TARGET/release/nc-uefi.efi" "$OUT/iso/EFI/BOOT/BOOTX64.EFI" || true
    fi
}

build_iso() {
    select_iso_tool
    mkdir -p "$OUT/iso/boot"
    cat "$OUT/stage1.bin" "$OUT/stage2.bin" "$OUT/kernel.bin" > "$OUT/iso/boot/ncboot.bin"
    build_uefi_optional
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

ascii_clean() {
    sed 's/ğ/g/g; s/Ğ/G/g; s/ü/u/g; s/Ü/U/g; s/ş/s/g; s/Ş/S/g; s/ı/i/g; s/İ/I/g; s/ö/o/g; s/Ö/O/g; s/ç/c/g; s/Ç/C/g' | tr -cd '\11\12\40-\176'
}

start_radio_bridge() {
    rm -f "$BUILD/radio.in" "$BUILD/radio.out"
    mkfifo "$BUILD/radio.in" "$BUILD/radio.out"
    (
        exec 3<>"$BUILD/radio.out"
        exec 4<>"$BUILD/radio.in"

        send_hw_info() {
            local w_hw="802.11 Kablosuz Ag Denetleyicisi"
            if command -v lspci >/dev/null 2>&1; then
                local found_w
                found_w=$(lspci | grep -i -E 'network controller|wireless|wi-fi|802\.11' | head -n 1 | cut -d: -f3- | sed 's/^ *//' | ascii_clean)
                if [ -n "$found_w" ]; then
                    w_hw="$found_w"
                fi
            fi
            local b_hw="Bluetooth HCI 5.x Denetleyici (hci0)"
            if command -v bluetoothctl >/dev/null 2>&1; then
                local ctrl_name
                ctrl_name=$(timeout 1 bluetoothctl list 2>/dev/null | head -n 1 | awk '{$1=""; $2=""; print $0}' | sed 's/^ *//' | ascii_clean || true)
                if [ -n "$ctrl_name" ]; then
                    b_hw="Bluetooth HCI: $ctrl_name"
                fi
            fi
            local w_on="1"
            if command -v nmcli >/dev/null 2>&1; then
                local r_state
                r_state=$(nmcli radio wifi 2>/dev/null || echo "enabled")
                [ "$r_state" = "disabled" ] && w_on="0"
            fi
            printf "H|%s|%s|%s|1\n" "$w_hw" "$b_hw" "$w_on" >&4
        }

        scan_host_wifi() {
            printf "W_CLEAR\n" >&4
            if command -v nmcli >/dev/null 2>&1; then
                nmcli -t -f IN-USE,SSID,CHAN,FREQ,SIGNAL,SECURITY dev wifi list --rescan auto 2>/dev/null | ascii_clean | while IFS=: read -r inuse ssid chan freq sig sec; do
                    [ -z "$ssid" ] && continue
                    [ "$ssid" = "--" ] && continue
                    local active="0"
                    [ "$inuse" = "*" ] && active="1"
                    [ -z "$sec" ] && sec="Acik"
                    printf "W|%s|%s|%s|%s|%s|%s\n" "$active" "$ssid" "${chan:-1}" "${freq:-2412 MHz}" "${sig:-50}" "$sec" >&4
                done
            fi
            printf "W_END\n" >&4
        }

        scan_host_bt() {
            printf "B_CLEAR\n" >&4
            if command -v bluetoothctl >/dev/null 2>&1; then
                timeout 1 bluetoothctl --timeout 1 scan on >/dev/null 2>&1 || true
                bluetoothctl devices 2>/dev/null | ascii_clean | while read -r tag mac rest; do
                    [ "$tag" != "Device" ] && continue
                    [ -z "$mac" ] && continue
                    local info
                    info=$(bluetoothctl info "$mac" 2>/dev/null | ascii_clean || true)
                    local conn="0"
                    echo "$info" | grep -q "Connected: yes" && conn="1"
                    local icon_kind="Bluetooth Cihaz"
                    if echo "$info" | grep -q -i "audio"; then
                        icon_kind="Ses / Kulaklik"
                    elif echo "$info" | grep -q -i "phone"; then
                        icon_kind="Akilli Telefon"
                    elif echo "$info" | grep -q -i "keyboard"; then
                        icon_kind="Kablosuz Klavye"
                    elif echo "$info" | grep -q -i "mouse"; then
                        icon_kind="Kablosuz Fare"
                    fi
                    local batt
                    batt=$(echo "$info" | grep "Battery Percentage" | sed -E 's/.*\(([0-9]+)\).*/\1/' | head -n 1 || echo "0")
                    [ -z "$batt" ] && batt="0"
                    printf "B|%s|%s|%s|%s|%s\n" "$conn" "$mac" "$icon_kind" "$batt" "${rest:-$mac}" >&4
                done
            fi
            printf "B_END\n" >&4
        }

        while IFS= read -r cmd <&3; do
            cmd=$(echo "$cmd" | tr -d '\r')
            case "$cmd" in
                CMD:INIT)
                    send_hw_info
                    ;;
                CMD:WIFI_SCAN)
                    scan_host_wifi
                    ;;
                CMD:WIFI_ON)
                    command -v nmcli >/dev/null 2>&1 && nmcli radio wifi on >/dev/null 2>&1 || true
                    send_hw_info
                    scan_host_wifi
                    ;;
                CMD:WIFI_OFF)
                    command -v nmcli >/dev/null 2>&1 && nmcli radio wifi off >/dev/null 2>&1 || true
                    send_hw_info
                    ;;
                CMD:WIFI_CONN\|*)
                    IFS='|' read -r _ target_ssid target_pass <<< "$cmd"
                    if command -v nmcli >/dev/null 2>&1; then
                        if [ -n "$target_pass" ]; then
                            nmcli dev wifi connect "$target_ssid" password "$target_pass" >/dev/null 2>&1 && \
                                printf "MSG|'%s' agina basariyla baglanildi.\n" "$target_ssid" >&4 || \
                                printf "MSG|'%s' baglantisi basarisiz oldu (Parolayi kontrol edin).\n" "$target_ssid" >&4
                        else
                            nmcli dev wifi connect "$target_ssid" >/dev/null 2>&1 && \
                                printf "MSG|'%s' agina basariyla baglanildi.\n" "$target_ssid" >&4 || \
                                printf "MSG|'%s' baglantisi basarisiz oldu.\n" "$target_ssid" >&4
                        fi
                    fi
                    scan_host_wifi
                    ;;
                CMD:WIFI_DISC\|*)
                    IFS='|' read -r _ target_ssid <<< "$cmd"
                    if command -v nmcli >/dev/null 2>&1; then
                        nmcli con down "$target_ssid" >/dev/null 2>&1 || true
                        printf "MSG|'%s' ag baglantisi kesildi.\n" "$target_ssid" >&4
                    fi
                    scan_host_wifi
                    ;;
                CMD:BT_SCAN)
                    scan_host_bt
                    ;;
                CMD:BT_ON)
                    command -v bluetoothctl >/dev/null 2>&1 && bluetoothctl power on >/dev/null 2>&1 || true
                    scan_host_bt
                    ;;
                CMD:BT_OFF)
                    command -v bluetoothctl >/dev/null 2>&1 && bluetoothctl power off >/dev/null 2>&1 || true
                    ;;
                CMD:BT_CONN\|*)
                    IFS='|' read -r _ target_mac <<< "$cmd"
                    if command -v bluetoothctl >/dev/null 2>&1; then
                        bluetoothctl connect "$target_mac" >/dev/null 2>&1 && \
                            printf "MSG|%s Bluetooth cihazi baglandi.\n" "$target_mac" >&4 || \
                            printf "MSG|%s cihazi yanit vermedi.\n" "$target_mac" >&4
                    fi
                    scan_host_bt
                    ;;
                CMD:BT_DISC\|*)
                    IFS='|' read -r _ target_mac <<< "$cmd"
                    if command -v bluetoothctl >/dev/null 2>&1; then
                        bluetoothctl disconnect "$target_mac" >/dev/null 2>&1 || true
                        printf "MSG|%s cihaz baglantisi kesildi.\n" "$target_mac" >&4
                    fi
                    scan_host_bt
                    ;;
            esac
        done
    ) &
    RADIO_PID=$!
    trap 'kill "$RADIO_PID" 2>/dev/null || true' EXIT
}

run_qemu() {
    need qemu-system-x86_64
    local accel=()
    if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        accel=(-enable-kvm -cpu host)
    fi

    if [ ! -f "$BUILD/disk.img" ]; then
        echo "20 GB test diski olusturuluyor: $BUILD/disk.img"
        truncate -s 20G "$BUILD/disk.img"
    fi

    start_radio_bridge

    local sig=""
    if [ -f "$BUILD/disk.img" ]; then
        sig=$(dd if="$BUILD/disk.img" bs=1 count=7 skip=3 2>/dev/null | tr -d '\0' || true)
    fi

   if [ "$sig" = "NETIZEN" ]; then
        echo "Kurulu NetizenCore algilandi: Yeni cekirdek diske yaziliyor ve baslatiliyor..."
        dd if="$OUT/stage2.bin" of="$BUILD/disk.img" bs=512 seek=2048 conv=notrunc status=none
        dd if="$OUT/kernel.bin" of="$BUILD/disk.img" bs=512 seek=2056 conv=notrunc status=none
        qemu-system-x86_64 \
            ${accel[@]+"${accel[@]}"} \
            -m 256 \
            -drive file="$BUILD/disk.img",format=raw,if=ide,index=0,media=disk \
            -boot order=c \
            -netdev user,id=net0 \
            -device e1000,netdev=net0 \
            -vga std \
            -serial stdio \
            -chardev pipe,id=ncradio,path="$BUILD/radio" \
            -serial chardev:ncradio \
            -debugcon "file:$BUILD/debugcon.log" \
            -device isa-debug-exit,iobase=0xf4,iosize=0x04
    else
        echo "Kurulum medyasi (ISO) ile baslatiliyor (kurulum bitince diskten acilacak)..."
        qemu-system-x86_64 \
            ${accel[@]+"${accel[@]}"} \
            -m 256 \
            -drive file="$BUILD/disk.img",format=raw,if=ide,index=0,media=disk \
            -cdrom "$ISO" \
            -boot once=d,order=c \
            -netdev user,id=net0 \
            -device e1000,netdev=net0 \
            -vga std \
            -serial stdio \
            -chardev pipe,id=ncradio,path="$BUILD/radio" \
            -serial chardev:ncradio \
            -debugcon "file:$BUILD/debugcon.log" \
            -device isa-debug-exit,iobase=0xf4,iosize=0x04
    fi
}

# build.sh içinde run_vbox() fonksiyonunu bununla değiştir:
build_drivers() {
    mkdir -p "$ROOT/drivers" "$BUILD/drivers"
    cat > "$ROOT/drivers/generic-display.efra.src" << 'EOF'
id=efra.display.generic
name=Netizen VBE/GOP Generic Turbo Framebuffer
version=2.1.0
file=generic-display.efra
class=display
vendor_id=0xFFFF
device_id=0xFFFF
pci_class=0x03
pci_subclass=0x00
write_combining=true
dma_burst=true
EOF
    printf "EFRA\x01\x00\x00\x00" > "$BUILD/drivers/generic-display.efra"
    echo "Harici .efra derleme alani hazir: $BUILD/drivers/generic-display.efra (ve cekirdek ici EfraCompiler aktif)"
}

run_vbox() {
    need VBoxManage
    if ! VBoxManage showvminfo "$VM_NAME" >/dev/null 2>&1; then
        VBoxManage createvm --name "$VM_NAME" --ostype Other_64 --register
        VBoxManage modifyvm "$VM_NAME" --memory 512 --vram 64 --firmware bios \
            --graphicscontroller vboxvga --nic1 nat --nictype1 82540EM \
            --boot1 dvd --boot2 disk --boot3 none --boot4 none
        VBoxManage storagectl "$VM_NAME" --name IDE --add ide
    fi
    if [ ! -f "$BUILD/vbox-disk.vdi" ]; then
        VBoxManage createmedium disk --filename "$BUILD/vbox-disk.vdi" --size 20480 --format VDI
    fi
    VBoxManage storageattach "$VM_NAME" --storagectl IDE --port 0 --device 0 \
        --type hdd --medium "$BUILD/vbox-disk.vdi"
    VBoxManage storageattach "$VM_NAME" --storagectl IDE --port 1 --device 0 \
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
    drivers)
        build_drivers
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
