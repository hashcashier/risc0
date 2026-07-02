const P: u32 = 2013265921u;
const M: u32 = 2281701377u;
const WORKGROUP_DISPATCH_STRIDE: u32 = 65535u;
const WORKGROUP_SIZE: u32 = 256u;
const FUSED_BITS: u32 = 10u;
const FUSED_BLOCK_SIZE: u32 = 1024u;

struct ElemBuffer {
    data: array<u32>,
};

struct Params {
    out_size: u32,
    row_count: u32,
    output_base: u32,
    twiddles_base: u32,
    n_bits: u32,
    blocks_per_row: u32,
    offsets_per_tile: u32,
    tiles_per_row: u32,
    total_tiles: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read_write> output: ElemBuffer;
@group(0) @binding(1) var<storage, read> twiddles: ElemBuffer;
@group(0) @binding(2) var<uniform> params: Params;

var<workgroup> scratch: array<u32, 1024>;

fn add(lhs: u32, rhs: u32) -> u32 {
    let sum = lhs + rhs;
    if (sum >= P) {
        return sum - P;
    }
    return sum;
}

fn sub(lhs: u32, rhs: u32) -> u32 {
    if (lhs >= rhs) {
        return lhs - rhs;
    }
    return lhs + P - rhs;
}

fn mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
    let lhs_lo = lhs & 0xffffu;
    let lhs_hi = lhs >> 16u;
    let rhs_lo = rhs & 0xffffu;
    let rhs_hi = rhs >> 16u;

    let p0 = lhs_lo * rhs_lo;
    let p1 = lhs_hi * rhs_lo;
    let p2 = lhs_lo * rhs_hi;
    let p3 = lhs_hi * rhs_hi;

    let carry = (p0 >> 16u) + (p1 & 0xffffu) + (p2 & 0xffffu);
    let lo = (p0 & 0xffffu) | ((carry & 0xffffu) << 16u);
    let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (carry >> 16u);
    return vec2<u32>(lo, hi);
}

fn mul(lhs: u32, rhs: u32) -> u32 {
    let product = mul_wide(lhs, rhs);
    let low = 0u - product.x;
    let red = M * low;
    let red_product = mul_wide(red, P);
    var ret = product.y + red_product.y;
    if (product.x + red_product.x < product.x) {
        ret = ret + 1u;
    }
    if (ret >= P) {
        return ret - P;
    }
    return ret;
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let tile_linear = workgroup_id.x + workgroup_id.y * WORKGROUP_DISPATCH_STRIDE;
    if (tile_linear >= params.total_tiles) {
        return;
    }
    if (
        params.blocks_per_row == 0u ||
        params.offsets_per_tile == 0u ||
        params.tiles_per_row == 0u
    ) {
        return;
    }

    let total_elems = params.blocks_per_row * params.offsets_per_tile;
    if (total_elems > FUSED_BLOCK_SIZE) {
        return;
    }

    let row = tile_linear / params.tiles_per_row;
    if (row >= params.row_count) {
        return;
    }
    let tile = tile_linear - row * params.tiles_per_row;
    let intra_base = tile * params.offsets_per_tile;
    let row_base = params.output_base + row * params.out_size;

    var i = local_id.x;
    loop {
        if (i >= total_elems) {
            break;
        }
        let lane = i / params.blocks_per_row;
        let block = i - lane * params.blocks_per_row;
        let intra = intra_base + lane;
        scratch[i] = output.data[row_base + block * FUSED_BLOCK_SIZE + intra];
        i = i + WORKGROUP_SIZE;
    }
    workgroupBarrier();

    var stage = FUSED_BITS + 1u;
    loop {
        if (stage > params.n_bits) {
            break;
        }
        let t = stage - FUSED_BITS;
        let block_s_size = 1u << (t - 1u);
        let pairs_per_lane = params.blocks_per_row >> 1u;
        let total_pairs = pairs_per_lane * params.offsets_per_tile;
        var pair_linear = local_id.x;
        loop {
            if (pair_linear >= total_pairs) {
                break;
            }
            let lane = pair_linear / pairs_per_lane;
            let pair = pair_linear - lane * pairs_per_lane;
            let g = pair / block_s_size;
            let block_s = pair - g * block_s_size;
            let idx1 = lane * params.blocks_per_row + g * 2u * block_s_size + block_s;
            let idx2 = idx1 + block_s_size;
            let intra = intra_base + lane;
            let s_original = block_s * FUSED_BLOCK_SIZE + intra;
            let stage_base = (1u << (stage - 1u)) - 1u;
            let cur_mul = twiddles.data[params.twiddles_base + stage_base + s_original];
            let a = scratch[idx1];
            let b = scratch[idx2];
            let b_mul = mul(b, cur_mul);
            scratch[idx1] = add(a, b_mul);
            scratch[idx2] = sub(a, b_mul);
            pair_linear = pair_linear + WORKGROUP_SIZE;
        }
        workgroupBarrier();
        stage = stage + 1u;
    }

    i = local_id.x;
    loop {
        if (i >= total_elems) {
            break;
        }
        let lane = i / params.blocks_per_row;
        let block = i - lane * params.blocks_per_row;
        let intra = intra_base + lane;
        output.data[row_base + block * FUSED_BLOCK_SIZE + intra] = scratch[i];
        i = i + WORKGROUP_SIZE;
    }
}
