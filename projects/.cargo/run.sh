#!/bin/bash
DIR=`dirname $0`
OUTPUT=`dirname $1`
KERNEL=`readlink -f $1`


rm -rf ${OUTPUT}/target/grub_files/
mkdir -p ${OUTPUT}/target/grub_files/boot/grub
cp ${1} ${OUTPUT}/target/grub_files/boot/keos
cp ${DIR}/grub.cfg ${OUTPUT}/target/grub_files/boot/grub/
grub-mkrescue /usr/lib/grub/i386-pc -o ${OUTPUT}/target/kernel.iso ${OUTPUT}/target/grub_files

mv ${1} keos_kernel

if [[ ! -z "${GDB}" ]]; then
    echo "target remote ${REMOTE:-0}:1234" > .gdbinit
    echo "symbol-file keos_kernel" >> .gdbinit
    echo "set print frame-arguments all" >> .gdbinit

    echo "c" >> ${DIR}/.gdbinit
    GDB='-S'
fi;

MP=${QEMU_SMP_COUNT:-4}
MEM=${QEMU_MEM_SIZE:-256}

cat /proc/cpuinfo | egrep "vmx|svm" > /dev/null
if [ $? -eq 0 ]; then
    QEMU_CPU_TYPE="-cpu host${QEMU_CPU_OPT} -enable-kvm"
else
    QEMU_CPU_TYPE="-cpu qemu64${QEMU_CPU_OPT}"
fi;

exec qemu-system-x86_64 \
    -nographic --boot d \
    -cdrom ${OUTPUT}/target/kernel.iso \
    -device virtio-blk-pci,drive=kernel -drive format=raw,if=none,file=keos_kernel,id=kernel,cache=none,readonly \
    -device virtio-blk-pci,drive=disk -drive format=raw,if=none,file=blk.bin,id=disk,cache=none \
    ${QEMU_CPU_TYPE} ${GDB} -s \
    -smp ${MP} -m ${MEM} -serial mon:stdio -no-reboot
