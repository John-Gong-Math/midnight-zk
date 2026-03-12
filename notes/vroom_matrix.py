import re
import subprocess
import json
from pathlib import Path

repo = Path('/home/zhiyonggong/midnight-zk')
files = [
    'vroom-msm-sys/src/lib.rs',
    'vroom-msm-sys/src/wrapper.cpp',
    'vroom-msm-sys/src/wrapper_msm.cpp',
    'vroom-msm-sys/src/wrapper_types.hpp',
]

def sh(cmd: str):
    return subprocess.run(['bash', '-lc', cmd], text=True, capture_output=True)

def reset():
    p = sh(f"cd {repo} && git checkout -- {' '.join(files)}")
    if p.returncode != 0:
        raise RuntimeError(p.stderr)

def patch_contiguous():
    wt = repo / 'vroom-msm-sys/src/wrapper_types.hpp'
    wc = repo / 'vroom-msm-sys/src/wrapper.cpp'
    s = wt.read_text()
    if '#include <array>' not in s:
        s = s.replace('#include <vector>', '#include <array>\n#include <vector>')
    s = s.replace('std::vector<std::vector<uint8_t>> data;', 'std::vector<std::array<uint8_t, 32>> data;')
    wt.write_text(s)

    s = wc.read_text()
    s = s.replace('        sc->data[i].resize(32);\n', '')
    wc.write_text(s)

def patch_void_return():
    wm = repo / 'vroom-msm-sys/src/wrapper_msm.cpp'
    lib = repo / 'vroom-msm-sys/src/lib.rs'

    s = wm.read_text()
    s = s.replace('uint64_t vroom_g1_msm', 'void vroom_g1_msm')
    s = s.replace('uint64_t vroom_g1_msm_parallel', 'void vroom_g1_msm_parallel')
    s = s.replace('auto result = msm(', '(void)msm(')
    s = s.replace('auto result = msm_parallel(', '(void)msm_parallel(')
    s = s.replace('    BigInt z = ctx->ring.to_bigint(result.z);\n', '')
    s = s.replace('    return z.to_ulong();\n', '')
    wm.write_text(s)

    s = lib.read_text()
    s = s.replace(') -> u64;', ');')
    lib.write_text(s)

def run_case(name: str):
    cmd = (
        f"cd {repo} && source $HOME/.cargo/env && "
        "NPOINTS=1048576 REPEATS=1 cargo bench -p midnight-curves --bench vroom_ffi_direct -- --nocapture"
    )
    p = sh(cmd)
    out = p.stdout + '\n' + p.stderr
    if p.returncode != 0:
        return {'case': name, 'ok': False, 'tail': out[-3000:]}

    def m(rx):
        mm = re.search(rx, out)
        return float(mm.group(1)) if mm else None

    return {
        'case': name,
        'ok': True,
        'setup_ms': m(r'setup:\s+([0-9.]+) ms'),
        'single_best_ms': m(r'vroom_ffi_single: best=([0-9.]+) ms'),
        'parallel_best_ms': m(r'vroom_ffi_parallel: best=([0-9.]+) ms'),
    }

cases = []

reset()
cases.append(run_case('baseline'))

reset()
patch_contiguous()
cases.append(run_case('contiguous_scalars'))

reset()
patch_void_return()
cases.append(run_case('void_return'))

reset()
patch_contiguous()
patch_void_return()
cases.append(run_case('both'))

print('MATRIX_RESULTS_JSON_START')
print(json.dumps(cases, indent=2))
print('MATRIX_RESULTS_JSON_END')
