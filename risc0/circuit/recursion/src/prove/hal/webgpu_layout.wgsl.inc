struct RecursionCheckedBytesInstLayout {
  evalPoint: Reg,
  keepCoeffs: Reg,
  keepUpperState: Reg,
  prepFull: Reg,
}
struct BoundLayout_RecursionCheckedBytesInstLayout {
  lyt: RecursionCheckedBytesInstLayout,
  buf: u32,
}
fn lookup_RecursionCheckedBytesInstLayout_evalPoint(b: BoundLayout_RecursionCheckedBytesInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.evalPoint, b.buf);
}
fn lookup_RecursionCheckedBytesInstLayout_keepCoeffs(b: BoundLayout_RecursionCheckedBytesInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.keepCoeffs, b.buf);
}
fn lookup_RecursionCheckedBytesInstLayout_keepUpperState(b: BoundLayout_RecursionCheckedBytesInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.keepUpperState, b.buf);
}
fn lookup_RecursionCheckedBytesInstLayout_prepFull(b: BoundLayout_RecursionCheckedBytesInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.prepFull, b.buf);
}
struct OneHotLayout {
  bitAndElem: Reg,
  bitOpShorts: Reg,
  nop: Reg,
  setGlobal: Reg,
  shaFini: Reg,
  shaInit: Reg,
  shaLoad: Reg,
  shaMix: Reg,
  womFini: Reg,
  womInit: Reg,
}
struct BoundLayout_OneHotLayout {
  lyt: OneHotLayout,
  buf: u32,
}
fn lookup_OneHotLayout_bitAndElem(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.bitAndElem, b.buf);
}
fn lookup_OneHotLayout_bitOpShorts(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.bitOpShorts, b.buf);
}
fn lookup_OneHotLayout_nop(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.nop, b.buf);
}
fn lookup_OneHotLayout_setGlobal(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.setGlobal, b.buf);
}
fn lookup_OneHotLayout_shaFini(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.shaFini, b.buf);
}
fn lookup_OneHotLayout_shaInit(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.shaInit, b.buf);
}
fn lookup_OneHotLayout_shaLoad(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.shaLoad, b.buf);
}
fn lookup_OneHotLayout_shaMix(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.shaMix, b.buf);
}
fn lookup_OneHotLayout_womFini(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.womFini, b.buf);
}
fn lookup_OneHotLayout_womInit(b: BoundLayout_OneHotLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.womInit, b.buf);
}
alias Reg3LayoutArray = array<Reg, 3>;
struct BoundLayout_Reg3LayoutArray {
  lyt: Reg3LayoutArray,
  buf: u32,
}
fn subscript_Reg3LayoutArray(b: BoundLayout_Reg3LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct RecursionMacroInstLayout {
  opcode: OneHotLayout,
  operand: Reg3LayoutArray,
}
struct BoundLayout_RecursionMacroInstLayout {
  lyt: RecursionMacroInstLayout,
  buf: u32,
}
fn lookup_RecursionMacroInstLayout_opcode(b: BoundLayout_RecursionMacroInstLayout) -> BoundLayout_OneHotLayout {
  return BoundLayout_OneHotLayout(b.lyt.opcode, b.buf);
}
fn lookup_RecursionMacroInstLayout_operand(b: BoundLayout_RecursionMacroInstLayout) -> BoundLayout_Reg3LayoutArray {
  return BoundLayout_Reg3LayoutArray(b.lyt.operand, b.buf);
}
struct RecursionMicroInstLayout {
  opcode: Reg,
  operand: Reg3LayoutArray,
}
struct BoundLayout_RecursionMicroInstLayout {
  lyt: RecursionMicroInstLayout,
  buf: u32,
}
fn lookup_RecursionMicroInstLayout_opcode(b: BoundLayout_RecursionMicroInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.opcode, b.buf);
}
fn lookup_RecursionMicroInstLayout_operand(b: BoundLayout_RecursionMicroInstLayout) -> BoundLayout_Reg3LayoutArray {
  return BoundLayout_Reg3LayoutArray(b.lyt.operand, b.buf);
}
alias RecursionMicroInstLayout3LayoutArray = array<RecursionMicroInstLayout, 3>;
struct BoundLayout_RecursionMicroInstLayout3LayoutArray {
  lyt: RecursionMicroInstLayout3LayoutArray,
  buf: u32,
}
fn subscript_RecursionMicroInstLayout3LayoutArray(b: BoundLayout_RecursionMicroInstLayout3LayoutArray, i: u32) -> BoundLayout_RecursionMicroInstLayout {
  return BoundLayout_RecursionMicroInstLayout(b.lyt[i], b.buf);
}
struct OneHot0Layout {
  c0: Reg,
  c1: Reg,
  c2: Reg,
  c3: Reg,
}
struct BoundLayout_OneHot0Layout {
  lyt: OneHot0Layout,
  buf: u32,
}
fn lookup_OneHot0Layout_c0(b: BoundLayout_OneHot0Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.c0, b.buf);
}
fn lookup_OneHot0Layout_c1(b: BoundLayout_OneHot0Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.c1, b.buf);
}
fn lookup_OneHot0Layout_c2(b: BoundLayout_OneHot0Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.c2, b.buf);
}
fn lookup_OneHot0Layout_c3(b: BoundLayout_OneHot0Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.c3, b.buf);
}
struct OneHot1Layout {
  g0: Reg,
  g1: Reg,
  g2: Reg,
}
struct BoundLayout_OneHot1Layout {
  lyt: OneHot1Layout,
  buf: u32,
}
fn lookup_OneHot1Layout_g0(b: BoundLayout_OneHot1Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.g0, b.buf);
}
fn lookup_OneHot1Layout_g1(b: BoundLayout_OneHot1Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.g1, b.buf);
}
fn lookup_OneHot1Layout_g2(b: BoundLayout_OneHot1Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.g2, b.buf);
}
alias Reg8LayoutArray = array<Reg, 8>;
struct BoundLayout_Reg8LayoutArray {
  lyt: Reg8LayoutArray,
  buf: u32,
}
fn subscript_Reg8LayoutArray(b: BoundLayout_Reg8LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct RecursionPoseidon2MemInstLayout {
  doMont: Reg,
  keepState: Reg,
  keepUpperState: Reg,
  prepFull: Reg,
  group: OneHot1Layout,
  inputs: Reg8LayoutArray,
}
struct BoundLayout_RecursionPoseidon2MemInstLayout {
  lyt: RecursionPoseidon2MemInstLayout,
  buf: u32,
}
fn lookup_RecursionPoseidon2MemInstLayout_doMont(b: BoundLayout_RecursionPoseidon2MemInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.doMont, b.buf);
}
fn lookup_RecursionPoseidon2MemInstLayout_keepState(b: BoundLayout_RecursionPoseidon2MemInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.keepState, b.buf);
}
fn lookup_RecursionPoseidon2MemInstLayout_keepUpperState(b: BoundLayout_RecursionPoseidon2MemInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.keepUpperState, b.buf);
}
fn lookup_RecursionPoseidon2MemInstLayout_prepFull(b: BoundLayout_RecursionPoseidon2MemInstLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.prepFull, b.buf);
}
fn lookup_RecursionPoseidon2MemInstLayout_group(b: BoundLayout_RecursionPoseidon2MemInstLayout) -> BoundLayout_OneHot1Layout {
  return BoundLayout_OneHot1Layout(b.lyt.group, b.buf);
}
fn lookup_RecursionPoseidon2MemInstLayout_inputs(b: BoundLayout_RecursionPoseidon2MemInstLayout) -> BoundLayout_Reg8LayoutArray {
  return BoundLayout_Reg8LayoutArray(b.lyt.inputs, b.buf);
}
struct MuxLayout {
  checkedBytes: RecursionCheckedBytesInstLayout,
  macroOps: RecursionMacroInstLayout,
  microOps: RecursionMicroInstLayout3LayoutArray,
  poseidon2Full: OneHot0Layout,
  poseidon2Load: RecursionPoseidon2MemInstLayout,
  poseidon2Store: RecursionPoseidon2MemInstLayout,
}
struct BoundLayout_MuxLayout {
  lyt: MuxLayout,
  buf: u32,
}
fn lookup_MuxLayout_checkedBytes(b: BoundLayout_MuxLayout) -> BoundLayout_RecursionCheckedBytesInstLayout {
  return BoundLayout_RecursionCheckedBytesInstLayout(b.lyt.checkedBytes, b.buf);
}
fn lookup_MuxLayout_macroOps(b: BoundLayout_MuxLayout) -> BoundLayout_RecursionMacroInstLayout {
  return BoundLayout_RecursionMacroInstLayout(b.lyt.macroOps, b.buf);
}
fn lookup_MuxLayout_microOps(b: BoundLayout_MuxLayout) -> BoundLayout_RecursionMicroInstLayout3LayoutArray {
  return BoundLayout_RecursionMicroInstLayout3LayoutArray(b.lyt.microOps, b.buf);
}
fn lookup_MuxLayout_poseidon2Full(b: BoundLayout_MuxLayout) -> BoundLayout_OneHot0Layout {
  return BoundLayout_OneHot0Layout(b.lyt.poseidon2Full, b.buf);
}
fn lookup_MuxLayout_poseidon2Load(b: BoundLayout_MuxLayout) -> BoundLayout_RecursionPoseidon2MemInstLayout {
  return BoundLayout_RecursionPoseidon2MemInstLayout(b.lyt.poseidon2Load, b.buf);
}
fn lookup_MuxLayout_poseidon2Store(b: BoundLayout_MuxLayout) -> BoundLayout_RecursionPoseidon2MemInstLayout {
  return BoundLayout_RecursionPoseidon2MemInstLayout(b.lyt.poseidon2Store, b.buf);
}
struct OneHot2Layout {
  checkedBytes: Reg,
  macroOps: Reg,
  microOps: Reg,
  poseidon2Full: Reg,
  poseidon2Load: Reg,
  poseidon2Partial: Reg,
  poseidon2Store: Reg,
}
struct BoundLayout_OneHot2Layout {
  lyt: OneHot2Layout,
  buf: u32,
}
fn lookup_OneHot2Layout_checkedBytes(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.checkedBytes, b.buf);
}
fn lookup_OneHot2Layout_macroOps(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.macroOps, b.buf);
}
fn lookup_OneHot2Layout_microOps(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.microOps, b.buf);
}
fn lookup_OneHot2Layout_poseidon2Full(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.poseidon2Full, b.buf);
}
fn lookup_OneHot2Layout_poseidon2Load(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.poseidon2Load, b.buf);
}
fn lookup_OneHot2Layout_poseidon2Partial(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.poseidon2Partial, b.buf);
}
fn lookup_OneHot2Layout_poseidon2Store(b: BoundLayout_OneHot2Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.poseidon2Store, b.buf);
}
struct RecursionCodeLayout {
  writeAddr: Reg,
  inst: MuxLayout,
  select: OneHot2Layout,
}
struct BoundLayout_RecursionCodeLayout {
  lyt: RecursionCodeLayout,
  buf: u32,
}
fn lookup_RecursionCodeLayout_writeAddr(b: BoundLayout_RecursionCodeLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.writeAddr, b.buf);
}
fn lookup_RecursionCodeLayout_inst(b: BoundLayout_RecursionCodeLayout) -> BoundLayout_MuxLayout {
  return BoundLayout_MuxLayout(b.lyt.inst, b.buf);
}
fn lookup_RecursionCodeLayout_select(b: BoundLayout_RecursionCodeLayout) -> BoundLayout_OneHot2Layout {
  return BoundLayout_OneHot2Layout(b.lyt.select, b.buf);
}
struct RecursionTopLayout {
  code: RecursionCodeLayout,
  mux: RecursionMicroInstLayout3LayoutArray,
}
struct BoundLayout_RecursionTopLayout {
  lyt: RecursionTopLayout,
  buf: u32,
}
fn lookup_RecursionTopLayout_code(b: BoundLayout_RecursionTopLayout) -> BoundLayout_RecursionCodeLayout {
  return BoundLayout_RecursionCodeLayout(b.lyt.code, b.buf);
}
fn lookup_RecursionTopLayout_mux(b: BoundLayout_RecursionTopLayout) -> BoundLayout_RecursionMicroInstLayout3LayoutArray {
  return BoundLayout_RecursionMicroInstLayout3LayoutArray(b.lyt.mux, b.buf);
}
const kCodeLayout: RecursionTopLayout = RecursionTopLayout(RecursionCodeLayout(Reg(0u), MuxLayout(RecursionCheckedBytesInstLayout(Reg(8u), Reg(9u), Reg(10u), Reg(11u)), RecursionMacroInstLayout(OneHotLayout(Reg(11u), Reg(12u), Reg(8u), Reg(17u), Reg(14u), Reg(13u), Reg(15u), Reg(16u), Reg(10u), Reg(9u)), Reg3LayoutArray(Reg(18u), Reg(19u), Reg(20u))), RecursionMicroInstLayout3LayoutArray(RecursionMicroInstLayout(Reg(8u), Reg3LayoutArray(Reg(9u), Reg(10u), Reg(11u))), RecursionMicroInstLayout(Reg(12u), Reg3LayoutArray(Reg(13u), Reg(14u), Reg(15u))), RecursionMicroInstLayout(Reg(16u), Reg3LayoutArray(Reg(17u), Reg(18u), Reg(19u)))), OneHot0Layout(Reg(8u), Reg(9u), Reg(10u), Reg(11u)), RecursionPoseidon2MemInstLayout(Reg(8u), Reg(9u), Reg(10u), Reg(11u), OneHot1Layout(Reg(12u), Reg(13u), Reg(14u)), Reg8LayoutArray(Reg(15u), Reg(16u), Reg(17u), Reg(18u), Reg(19u), Reg(20u), Reg(21u), Reg(22u))), RecursionPoseidon2MemInstLayout(Reg(8u), Reg(9u), Reg(10u), Reg(11u), OneHot1Layout(Reg(12u), Reg(13u), Reg(14u)), Reg8LayoutArray(Reg(15u), Reg(16u), Reg(17u), Reg(18u), Reg(19u), Reg(20u), Reg(21u), Reg(22u)))), OneHot2Layout(Reg(7u), Reg(2u), Reg(1u), Reg(4u), Reg(3u), Reg(5u), Reg(6u))), RecursionMicroInstLayout3LayoutArray(RecursionMicroInstLayout(Reg(8u), Reg3LayoutArray(Reg(9u), Reg(10u), Reg(11u))), RecursionMicroInstLayout(Reg(12u), Reg3LayoutArray(Reg(13u), Reg(14u), Reg(15u))), RecursionMicroInstLayout(Reg(16u), Reg3LayoutArray(Reg(17u), Reg(18u), Reg(19u)))));
alias Reg4LayoutArray = array<Reg, 4>;
struct BoundLayout_Reg4LayoutArray {
  lyt: Reg4LayoutArray,
  buf: u32,
}
fn subscript_Reg4LayoutArray(b: BoundLayout_Reg4LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct RecursionImplWomPlonkElementLayout {
  addr: Reg,
  data: Reg4LayoutArray,
}
struct BoundLayout_RecursionImplWomPlonkElementLayout {
  lyt: RecursionImplWomPlonkElementLayout,
  buf: u32,
}
fn lookup_RecursionImplWomPlonkElementLayout_addr(b: BoundLayout_RecursionImplWomPlonkElementLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.addr, b.buf);
}
fn lookup_RecursionImplWomPlonkElementLayout_data(b: BoundLayout_RecursionImplWomPlonkElementLayout) -> BoundLayout_Reg4LayoutArray {
  return BoundLayout_Reg4LayoutArray(b.lyt.data, b.buf);
}
alias Reg31LayoutArray = array<Reg, 31>;
struct BoundLayout_Reg31LayoutArray {
  lyt: Reg31LayoutArray,
  buf: u32,
}
fn subscript_Reg31LayoutArray(b: BoundLayout_Reg31LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct RecursionBitAndElemLayout {
  inA: RecursionImplWomPlonkElementLayout,
  inB: RecursionImplWomPlonkElementLayout,
  out_: RecursionImplWomPlonkElementLayout,
  bitsA: Reg31LayoutArray,
  bitsB: Reg31LayoutArray,
}
struct BoundLayout_RecursionBitAndElemLayout {
  lyt: RecursionBitAndElemLayout,
  buf: u32,
}
fn lookup_RecursionBitAndElemLayout_inA(b: BoundLayout_RecursionBitAndElemLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.inA, b.buf);
}
fn lookup_RecursionBitAndElemLayout_inB(b: BoundLayout_RecursionBitAndElemLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.inB, b.buf);
}
fn lookup_RecursionBitAndElemLayout_out_(b: BoundLayout_RecursionBitAndElemLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.out_, b.buf);
}
fn lookup_RecursionBitAndElemLayout_bitsA(b: BoundLayout_RecursionBitAndElemLayout) -> BoundLayout_Reg31LayoutArray {
  return BoundLayout_Reg31LayoutArray(b.lyt.bitsA, b.buf);
}
fn lookup_RecursionBitAndElemLayout_bitsB(b: BoundLayout_RecursionBitAndElemLayout) -> BoundLayout_Reg31LayoutArray {
  return BoundLayout_Reg31LayoutArray(b.lyt.bitsB, b.buf);
}
alias Reg32LayoutArray = array<Reg, 32>;
struct BoundLayout_Reg32LayoutArray {
  lyt: Reg32LayoutArray,
  buf: u32,
}
fn subscript_Reg32LayoutArray(b: BoundLayout_Reg32LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct RecursionBitOpShortsLayout {
  inA: RecursionImplWomPlonkElementLayout,
  inB: RecursionImplWomPlonkElementLayout,
  out_: RecursionImplWomPlonkElementLayout,
  bitsA: Reg32LayoutArray,
  bitsB: Reg32LayoutArray,
}
struct BoundLayout_RecursionBitOpShortsLayout {
  lyt: RecursionBitOpShortsLayout,
  buf: u32,
}
fn lookup_RecursionBitOpShortsLayout_inA(b: BoundLayout_RecursionBitOpShortsLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.inA, b.buf);
}
fn lookup_RecursionBitOpShortsLayout_inB(b: BoundLayout_RecursionBitOpShortsLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.inB, b.buf);
}
fn lookup_RecursionBitOpShortsLayout_out_(b: BoundLayout_RecursionBitOpShortsLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.out_, b.buf);
}
fn lookup_RecursionBitOpShortsLayout_bitsA(b: BoundLayout_RecursionBitOpShortsLayout) -> BoundLayout_Reg32LayoutArray {
  return BoundLayout_Reg32LayoutArray(b.lyt.bitsA, b.buf);
}
fn lookup_RecursionBitOpShortsLayout_bitsB(b: BoundLayout_RecursionBitOpShortsLayout) -> BoundLayout_Reg32LayoutArray {
  return BoundLayout_Reg32LayoutArray(b.lyt.bitsB, b.buf);
}
alias RecursionImplWomPlonkElementLayout4LayoutArray = array<RecursionImplWomPlonkElementLayout, 4>;
struct BoundLayout_RecursionImplWomPlonkElementLayout4LayoutArray {
  lyt: RecursionImplWomPlonkElementLayout4LayoutArray,
  buf: u32,
}
fn subscript_RecursionImplWomPlonkElementLayout4LayoutArray(b: BoundLayout_RecursionImplWomPlonkElementLayout4LayoutArray, i: u32) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt[i], b.buf);
}
struct RecursionSetGlobalLayout {
  select: Reg4LayoutArray,
  regs: RecursionImplWomPlonkElementLayout4LayoutArray,
}
struct BoundLayout_RecursionSetGlobalLayout {
  lyt: RecursionSetGlobalLayout,
  buf: u32,
}
fn lookup_RecursionSetGlobalLayout_select(b: BoundLayout_RecursionSetGlobalLayout) -> BoundLayout_Reg4LayoutArray {
  return BoundLayout_Reg4LayoutArray(b.lyt.select, b.buf);
}
fn lookup_RecursionSetGlobalLayout_regs(b: BoundLayout_RecursionSetGlobalLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout4LayoutArray {
  return BoundLayout_RecursionImplWomPlonkElementLayout4LayoutArray(b.lyt.regs, b.buf);
}
alias Reg2LayoutArray = array<Reg, 2>;
struct BoundLayout_Reg2LayoutArray {
  lyt: Reg2LayoutArray,
  buf: u32,
}
fn subscript_Reg2LayoutArray(b: BoundLayout_Reg2LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct RecursionShaCycleLayout {
  aCarryHi: Reg,
  aCarryLow: Reg,
  eCarryHigh: Reg,
  eCarryLow: Reg,
  wCarryHigh: Reg,
  wCarryLow: Reg,
  io0: RecursionImplWomPlonkElementLayout,
  io1: RecursionImplWomPlonkElementLayout,
  a: Reg32LayoutArray,
  aRaw: Reg2LayoutArray,
  e: Reg32LayoutArray,
  eRaw: Reg2LayoutArray,
  w: Reg32LayoutArray,
  wRaw: Reg2LayoutArray,
}
struct BoundLayout_RecursionShaCycleLayout {
  lyt: RecursionShaCycleLayout,
  buf: u32,
}
fn lookup_RecursionShaCycleLayout_aCarryHi(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.aCarryHi, b.buf);
}
fn lookup_RecursionShaCycleLayout_aCarryLow(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.aCarryLow, b.buf);
}
fn lookup_RecursionShaCycleLayout_eCarryHigh(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.eCarryHigh, b.buf);
}
fn lookup_RecursionShaCycleLayout_eCarryLow(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.eCarryLow, b.buf);
}
fn lookup_RecursionShaCycleLayout_wCarryHigh(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.wCarryHigh, b.buf);
}
fn lookup_RecursionShaCycleLayout_wCarryLow(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.wCarryLow, b.buf);
}
fn lookup_RecursionShaCycleLayout_io0(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.io0, b.buf);
}
fn lookup_RecursionShaCycleLayout_io1(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.io1, b.buf);
}
fn lookup_RecursionShaCycleLayout_a(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg32LayoutArray {
  return BoundLayout_Reg32LayoutArray(b.lyt.a, b.buf);
}
fn lookup_RecursionShaCycleLayout_aRaw(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg2LayoutArray {
  return BoundLayout_Reg2LayoutArray(b.lyt.aRaw, b.buf);
}
fn lookup_RecursionShaCycleLayout_e(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg32LayoutArray {
  return BoundLayout_Reg32LayoutArray(b.lyt.e, b.buf);
}
fn lookup_RecursionShaCycleLayout_eRaw(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg2LayoutArray {
  return BoundLayout_Reg2LayoutArray(b.lyt.eRaw, b.buf);
}
fn lookup_RecursionShaCycleLayout_w(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg32LayoutArray {
  return BoundLayout_Reg32LayoutArray(b.lyt.w, b.buf);
}
fn lookup_RecursionShaCycleLayout_wRaw(b: BoundLayout_RecursionShaCycleLayout) -> BoundLayout_Reg2LayoutArray {
  return BoundLayout_Reg2LayoutArray(b.lyt.wRaw, b.buf);
}
struct Mux3Layout {
  bitAndElem: RecursionBitAndElemLayout,
  bitOpShorts: RecursionBitOpShortsLayout,
  setGlobal: RecursionSetGlobalLayout,
  shaFini: RecursionShaCycleLayout,
  shaInit: RecursionShaCycleLayout,
  shaLoad: RecursionShaCycleLayout,
  shaMix: RecursionShaCycleLayout,
}
struct BoundLayout_Mux3Layout {
  lyt: Mux3Layout,
  buf: u32,
}
fn lookup_Mux3Layout_bitAndElem(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionBitAndElemLayout {
  return BoundLayout_RecursionBitAndElemLayout(b.lyt.bitAndElem, b.buf);
}
fn lookup_Mux3Layout_bitOpShorts(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionBitOpShortsLayout {
  return BoundLayout_RecursionBitOpShortsLayout(b.lyt.bitOpShorts, b.buf);
}
fn lookup_Mux3Layout_setGlobal(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionSetGlobalLayout {
  return BoundLayout_RecursionSetGlobalLayout(b.lyt.setGlobal, b.buf);
}
fn lookup_Mux3Layout_shaFini(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionShaCycleLayout {
  return BoundLayout_RecursionShaCycleLayout(b.lyt.shaFini, b.buf);
}
fn lookup_Mux3Layout_shaInit(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionShaCycleLayout {
  return BoundLayout_RecursionShaCycleLayout(b.lyt.shaInit, b.buf);
}
fn lookup_Mux3Layout_shaLoad(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionShaCycleLayout {
  return BoundLayout_RecursionShaCycleLayout(b.lyt.shaLoad, b.buf);
}
fn lookup_Mux3Layout_shaMix(b: BoundLayout_Mux3Layout) -> BoundLayout_RecursionShaCycleLayout {
  return BoundLayout_RecursionShaCycleLayout(b.lyt.shaMix, b.buf);
}
struct OneHot4Layout {
  add: Reg,
  constop: Reg,
  eq: Reg,
  extract: Reg,
  inv: Reg,
  mixRng: Reg,
  mul: Reg,
  readIopBody: Reg,
  readIopHeader: Reg,
  select: Reg,
  sub: Reg,
}
struct BoundLayout_OneHot4Layout {
  lyt: OneHot4Layout,
  buf: u32,
}
fn lookup_OneHot4Layout_add(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.add, b.buf);
}
fn lookup_OneHot4Layout_constop(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.constop, b.buf);
}
fn lookup_OneHot4Layout_eq(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.eq, b.buf);
}
fn lookup_OneHot4Layout_extract(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.extract, b.buf);
}
fn lookup_OneHot4Layout_inv(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.inv, b.buf);
}
fn lookup_OneHot4Layout_mixRng(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.mixRng, b.buf);
}
fn lookup_OneHot4Layout_mul(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.mul, b.buf);
}
fn lookup_OneHot4Layout_readIopBody(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.readIopBody, b.buf);
}
fn lookup_OneHot4Layout_readIopHeader(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.readIopHeader, b.buf);
}
fn lookup_OneHot4Layout_select(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.select, b.buf);
}
fn lookup_OneHot4Layout_sub(b: BoundLayout_OneHot4Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.sub, b.buf);
}
struct RecursionMicroOpLayout {
  extra: Reg,
  decode: OneHot4Layout,
  in0: RecursionImplWomPlonkElementLayout,
  in1: RecursionImplWomPlonkElementLayout,
  out_: RecursionImplWomPlonkElementLayout,
}
struct BoundLayout_RecursionMicroOpLayout {
  lyt: RecursionMicroOpLayout,
  buf: u32,
}
fn lookup_RecursionMicroOpLayout_extra(b: BoundLayout_RecursionMicroOpLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.extra, b.buf);
}
fn lookup_RecursionMicroOpLayout_decode(b: BoundLayout_RecursionMicroOpLayout) -> BoundLayout_OneHot4Layout {
  return BoundLayout_OneHot4Layout(b.lyt.decode, b.buf);
}
fn lookup_RecursionMicroOpLayout_in0(b: BoundLayout_RecursionMicroOpLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.in0, b.buf);
}
fn lookup_RecursionMicroOpLayout_in1(b: BoundLayout_RecursionMicroOpLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.in1, b.buf);
}
fn lookup_RecursionMicroOpLayout_out_(b: BoundLayout_RecursionMicroOpLayout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.out_, b.buf);
}
alias RecursionMicroOpLayout3LayoutArray = array<RecursionMicroOpLayout, 3>;
struct BoundLayout_RecursionMicroOpLayout3LayoutArray {
  lyt: RecursionMicroOpLayout3LayoutArray,
  buf: u32,
}
fn subscript_RecursionMicroOpLayout3LayoutArray(b: BoundLayout_RecursionMicroOpLayout3LayoutArray, i: u32) -> BoundLayout_RecursionMicroOpLayout {
  return BoundLayout_RecursionMicroOpLayout(b.lyt[i], b.buf);
}
struct Mux5Layout {
  checkedBytes: RecursionImplWomPlonkElementLayout,
  macroOps: Mux3Layout,
  microOps: RecursionMicroOpLayout3LayoutArray,
}
struct BoundLayout_Mux5Layout {
  lyt: Mux5Layout,
  buf: u32,
}
fn lookup_Mux5Layout_checkedBytes(b: BoundLayout_Mux5Layout) -> BoundLayout_RecursionImplWomPlonkElementLayout {
  return BoundLayout_RecursionImplWomPlonkElementLayout(b.lyt.checkedBytes, b.buf);
}
fn lookup_Mux5Layout_macroOps(b: BoundLayout_Mux5Layout) -> BoundLayout_Mux3Layout {
  return BoundLayout_Mux3Layout(b.lyt.macroOps, b.buf);
}
fn lookup_Mux5Layout_microOps(b: BoundLayout_Mux5Layout) -> BoundLayout_RecursionMicroOpLayout3LayoutArray {
  return BoundLayout_RecursionMicroOpLayout3LayoutArray(b.lyt.microOps, b.buf);
}
const kDataLayout: Mux5Layout = Mux5Layout(RecursionImplWomPlonkElementLayout(Reg(5u), Reg4LayoutArray(Reg(6u), Reg(7u), Reg(8u), Reg(9u))), Mux3Layout(RecursionBitAndElemLayout(RecursionImplWomPlonkElementLayout(Reg(11u), Reg4LayoutArray(Reg(12u), Reg(13u), Reg(14u), Reg(15u))), RecursionImplWomPlonkElementLayout(Reg(16u), Reg4LayoutArray(Reg(17u), Reg(18u), Reg(19u), Reg(20u))), RecursionImplWomPlonkElementLayout(Reg(21u), Reg4LayoutArray(Reg(22u), Reg(23u), Reg(24u), Reg(25u))), Reg31LayoutArray(Reg(36u), Reg(38u), Reg(40u), Reg(42u), Reg(44u), Reg(46u), Reg(48u), Reg(50u), Reg(52u), Reg(54u), Reg(56u), Reg(58u), Reg(60u), Reg(62u), Reg(64u), Reg(66u), Reg(68u), Reg(70u), Reg(72u), Reg(74u), Reg(76u), Reg(78u), Reg(80u), Reg(82u), Reg(84u), Reg(86u), Reg(88u), Reg(90u), Reg(92u), Reg(94u), Reg(96u)), Reg31LayoutArray(Reg(37u), Reg(39u), Reg(41u), Reg(43u), Reg(45u), Reg(47u), Reg(49u), Reg(51u), Reg(53u), Reg(55u), Reg(57u), Reg(59u), Reg(61u), Reg(63u), Reg(65u), Reg(67u), Reg(69u), Reg(71u), Reg(73u), Reg(75u), Reg(77u), Reg(79u), Reg(81u), Reg(83u), Reg(85u), Reg(87u), Reg(89u), Reg(91u), Reg(93u), Reg(95u), Reg(97u))), RecursionBitOpShortsLayout(RecursionImplWomPlonkElementLayout(Reg(11u), Reg4LayoutArray(Reg(12u), Reg(13u), Reg(14u), Reg(15u))), RecursionImplWomPlonkElementLayout(Reg(16u), Reg4LayoutArray(Reg(17u), Reg(18u), Reg(19u), Reg(20u))), RecursionImplWomPlonkElementLayout(Reg(21u), Reg4LayoutArray(Reg(22u), Reg(23u), Reg(24u), Reg(25u))), Reg32LayoutArray(Reg(36u), Reg(37u), Reg(38u), Reg(39u), Reg(40u), Reg(41u), Reg(42u), Reg(43u), Reg(44u), Reg(45u), Reg(46u), Reg(47u), Reg(48u), Reg(49u), Reg(50u), Reg(51u), Reg(52u), Reg(53u), Reg(54u), Reg(55u), Reg(56u), Reg(57u), Reg(58u), Reg(59u), Reg(60u), Reg(61u), Reg(62u), Reg(63u), Reg(64u), Reg(65u), Reg(66u), Reg(67u)), Reg32LayoutArray(Reg(68u), Reg(69u), Reg(70u), Reg(71u), Reg(72u), Reg(73u), Reg(74u), Reg(75u), Reg(76u), Reg(77u), Reg(78u), Reg(79u), Reg(80u), Reg(81u), Reg(82u), Reg(83u), Reg(84u), Reg(85u), Reg(86u), Reg(87u), Reg(88u), Reg(89u), Reg(90u), Reg(91u), Reg(92u), Reg(93u), Reg(94u), Reg(95u), Reg(96u), Reg(97u), Reg(98u), Reg(99u))), RecursionSetGlobalLayout(Reg4LayoutArray(Reg(11u), Reg(12u), Reg(13u), Reg(14u)), RecursionImplWomPlonkElementLayout4LayoutArray(RecursionImplWomPlonkElementLayout(Reg(15u), Reg4LayoutArray(Reg(16u), Reg(17u), Reg(18u), Reg(19u))), RecursionImplWomPlonkElementLayout(Reg(20u), Reg4LayoutArray(Reg(21u), Reg(22u), Reg(23u), Reg(24u))), RecursionImplWomPlonkElementLayout(Reg(25u), Reg4LayoutArray(Reg(26u), Reg(27u), Reg(28u), Reg(29u))), RecursionImplWomPlonkElementLayout(Reg(30u), Reg4LayoutArray(Reg(31u), Reg(32u), Reg(33u), Reg(34u))))), RecursionShaCycleLayout(Reg(6u), Reg(5u), Reg(8u), Reg(7u), Reg(10u), Reg(9u), RecursionImplWomPlonkElementLayout(Reg(11u), Reg4LayoutArray(Reg(12u), Reg(13u), Reg(14u), Reg(15u))), RecursionImplWomPlonkElementLayout(Reg(16u), Reg4LayoutArray(Reg(17u), Reg(18u), Reg(19u), Reg(20u))), Reg32LayoutArray(Reg(26u), Reg(27u), Reg(28u), Reg(29u), Reg(30u), Reg(31u), Reg(32u), Reg(33u), Reg(34u), Reg(35u), Reg(36u), Reg(37u), Reg(38u), Reg(39u), Reg(40u), Reg(41u), Reg(42u), Reg(43u), Reg(44u), Reg(45u), Reg(46u), Reg(47u), Reg(48u), Reg(49u), Reg(50u), Reg(51u), Reg(52u), Reg(53u), Reg(54u), Reg(55u), Reg(56u), Reg(57u)), Reg2LayoutArray(Reg(58u), Reg(59u)), Reg32LayoutArray(Reg(60u), Reg(61u), Reg(62u), Reg(63u), Reg(64u), Reg(65u), Reg(66u), Reg(67u), Reg(68u), Reg(69u), Reg(70u), Reg(71u), Reg(72u), Reg(73u), Reg(74u), Reg(75u), Reg(76u), Reg(77u), Reg(78u), Reg(79u), Reg(80u), Reg(81u), Reg(82u), Reg(83u), Reg(84u), Reg(85u), Reg(86u), Reg(87u), Reg(88u), Reg(89u), Reg(90u), Reg(91u)), Reg2LayoutArray(Reg(92u), Reg(93u)), Reg32LayoutArray(Reg(94u), Reg(95u), Reg(96u), Reg(97u), Reg(98u), Reg(99u), Reg(100u), Reg(101u), Reg(102u), Reg(103u), Reg(104u), Reg(105u), Reg(106u), Reg(107u), Reg(108u), Reg(109u), Reg(110u), Reg(111u), Reg(112u), Reg(113u), Reg(114u), Reg(115u), Reg(116u), Reg(117u), Reg(118u), Reg(119u), Reg(120u), Reg(121u), Reg(122u), Reg(123u), Reg(124u), Reg(125u)), Reg2LayoutArray(Reg(126u), Reg(127u))), RecursionShaCycleLayout(Reg(6u), Reg(5u), Reg(8u), Reg(7u), Reg(10u), Reg(9u), RecursionImplWomPlonkElementLayout(Reg(11u), Reg4LayoutArray(Reg(12u), Reg(13u), Reg(14u), Reg(15u))), RecursionImplWomPlonkElementLayout(Reg(16u), Reg4LayoutArray(Reg(17u), Reg(18u), Reg(19u), Reg(20u))), Reg32LayoutArray(Reg(26u), Reg(27u), Reg(28u), Reg(29u), Reg(30u), Reg(31u), Reg(32u), Reg(33u), Reg(34u), Reg(35u), Reg(36u), Reg(37u), Reg(38u), Reg(39u), Reg(40u), Reg(41u), Reg(42u), Reg(43u), Reg(44u), Reg(45u), Reg(46u), Reg(47u), Reg(48u), Reg(49u), Reg(50u), Reg(51u), Reg(52u), Reg(53u), Reg(54u), Reg(55u), Reg(56u), Reg(57u)), Reg2LayoutArray(Reg(58u), Reg(59u)), Reg32LayoutArray(Reg(60u), Reg(61u), Reg(62u), Reg(63u), Reg(64u), Reg(65u), Reg(66u), Reg(67u), Reg(68u), Reg(69u), Reg(70u), Reg(71u), Reg(72u), Reg(73u), Reg(74u), Reg(75u), Reg(76u), Reg(77u), Reg(78u), Reg(79u), Reg(80u), Reg(81u), Reg(82u), Reg(83u), Reg(84u), Reg(85u), Reg(86u), Reg(87u), Reg(88u), Reg(89u), Reg(90u), Reg(91u)), Reg2LayoutArray(Reg(92u), Reg(93u)), Reg32LayoutArray(Reg(94u), Reg(95u), Reg(96u), Reg(97u), Reg(98u), Reg(99u), Reg(100u), Reg(101u), Reg(102u), Reg(103u), Reg(104u), Reg(105u), Reg(106u), Reg(107u), Reg(108u), Reg(109u), Reg(110u), Reg(111u), Reg(112u), Reg(113u), Reg(114u), Reg(115u), Reg(116u), Reg(117u), Reg(118u), Reg(119u), Reg(120u), Reg(121u), Reg(122u), Reg(123u), Reg(124u), Reg(125u)), Reg2LayoutArray(Reg(126u), Reg(127u))), RecursionShaCycleLayout(Reg(6u), Reg(5u), Reg(8u), Reg(7u), Reg(10u), Reg(9u), RecursionImplWomPlonkElementLayout(Reg(11u), Reg4LayoutArray(Reg(12u), Reg(13u), Reg(14u), Reg(15u))), RecursionImplWomPlonkElementLayout(Reg(16u), Reg4LayoutArray(Reg(17u), Reg(18u), Reg(19u), Reg(20u))), Reg32LayoutArray(Reg(26u), Reg(27u), Reg(28u), Reg(29u), Reg(30u), Reg(31u), Reg(32u), Reg(33u), Reg(34u), Reg(35u), Reg(36u), Reg(37u), Reg(38u), Reg(39u), Reg(40u), Reg(41u), Reg(42u), Reg(43u), Reg(44u), Reg(45u), Reg(46u), Reg(47u), Reg(48u), Reg(49u), Reg(50u), Reg(51u), Reg(52u), Reg(53u), Reg(54u), Reg(55u), Reg(56u), Reg(57u)), Reg2LayoutArray(Reg(58u), Reg(59u)), Reg32LayoutArray(Reg(60u), Reg(61u), Reg(62u), Reg(63u), Reg(64u), Reg(65u), Reg(66u), Reg(67u), Reg(68u), Reg(69u), Reg(70u), Reg(71u), Reg(72u), Reg(73u), Reg(74u), Reg(75u), Reg(76u), Reg(77u), Reg(78u), Reg(79u), Reg(80u), Reg(81u), Reg(82u), Reg(83u), Reg(84u), Reg(85u), Reg(86u), Reg(87u), Reg(88u), Reg(89u), Reg(90u), Reg(91u)), Reg2LayoutArray(Reg(92u), Reg(93u)), Reg32LayoutArray(Reg(94u), Reg(95u), Reg(96u), Reg(97u), Reg(98u), Reg(99u), Reg(100u), Reg(101u), Reg(102u), Reg(103u), Reg(104u), Reg(105u), Reg(106u), Reg(107u), Reg(108u), Reg(109u), Reg(110u), Reg(111u), Reg(112u), Reg(113u), Reg(114u), Reg(115u), Reg(116u), Reg(117u), Reg(118u), Reg(119u), Reg(120u), Reg(121u), Reg(122u), Reg(123u), Reg(124u), Reg(125u)), Reg2LayoutArray(Reg(126u), Reg(127u))), RecursionShaCycleLayout(Reg(6u), Reg(5u), Reg(8u), Reg(7u), Reg(10u), Reg(9u), RecursionImplWomPlonkElementLayout(Reg(11u), Reg4LayoutArray(Reg(12u), Reg(13u), Reg(14u), Reg(15u))), RecursionImplWomPlonkElementLayout(Reg(16u), Reg4LayoutArray(Reg(17u), Reg(18u), Reg(19u), Reg(20u))), Reg32LayoutArray(Reg(26u), Reg(27u), Reg(28u), Reg(29u), Reg(30u), Reg(31u), Reg(32u), Reg(33u), Reg(34u), Reg(35u), Reg(36u), Reg(37u), Reg(38u), Reg(39u), Reg(40u), Reg(41u), Reg(42u), Reg(43u), Reg(44u), Reg(45u), Reg(46u), Reg(47u), Reg(48u), Reg(49u), Reg(50u), Reg(51u), Reg(52u), Reg(53u), Reg(54u), Reg(55u), Reg(56u), Reg(57u)), Reg2LayoutArray(Reg(58u), Reg(59u)), Reg32LayoutArray(Reg(60u), Reg(61u), Reg(62u), Reg(63u), Reg(64u), Reg(65u), Reg(66u), Reg(67u), Reg(68u), Reg(69u), Reg(70u), Reg(71u), Reg(72u), Reg(73u), Reg(74u), Reg(75u), Reg(76u), Reg(77u), Reg(78u), Reg(79u), Reg(80u), Reg(81u), Reg(82u), Reg(83u), Reg(84u), Reg(85u), Reg(86u), Reg(87u), Reg(88u), Reg(89u), Reg(90u), Reg(91u)), Reg2LayoutArray(Reg(92u), Reg(93u)), Reg32LayoutArray(Reg(94u), Reg(95u), Reg(96u), Reg(97u), Reg(98u), Reg(99u), Reg(100u), Reg(101u), Reg(102u), Reg(103u), Reg(104u), Reg(105u), Reg(106u), Reg(107u), Reg(108u), Reg(109u), Reg(110u), Reg(111u), Reg(112u), Reg(113u), Reg(114u), Reg(115u), Reg(116u), Reg(117u), Reg(118u), Reg(119u), Reg(120u), Reg(121u), Reg(122u), Reg(123u), Reg(124u), Reg(125u)), Reg2LayoutArray(Reg(126u), Reg(127u)))), RecursionMicroOpLayout3LayoutArray(RecursionMicroOpLayout(Reg(101u), OneHot4Layout(Reg(91u), Reg(90u), Reg(95u), Reg(100u), Reg(94u), Reg(98u), Reg(93u), Reg(97u), Reg(96u), Reg(99u), Reg(92u)), RecursionImplWomPlonkElementLayout(Reg(5u), Reg4LayoutArray(Reg(6u), Reg(7u), Reg(8u), Reg(9u))), RecursionImplWomPlonkElementLayout(Reg(10u), Reg4LayoutArray(Reg(11u), Reg(12u), Reg(13u), Reg(14u))), RecursionImplWomPlonkElementLayout(Reg(15u), Reg4LayoutArray(Reg(16u), Reg(17u), Reg(18u), Reg(19u)))), RecursionMicroOpLayout(Reg(113u), OneHot4Layout(Reg(103u), Reg(102u), Reg(107u), Reg(112u), Reg(106u), Reg(110u), Reg(105u), Reg(109u), Reg(108u), Reg(111u), Reg(104u)), RecursionImplWomPlonkElementLayout(Reg(20u), Reg4LayoutArray(Reg(21u), Reg(22u), Reg(23u), Reg(24u))), RecursionImplWomPlonkElementLayout(Reg(25u), Reg4LayoutArray(Reg(26u), Reg(27u), Reg(28u), Reg(29u))), RecursionImplWomPlonkElementLayout(Reg(30u), Reg4LayoutArray(Reg(31u), Reg(32u), Reg(33u), Reg(34u)))), RecursionMicroOpLayout(Reg(125u), OneHot4Layout(Reg(115u), Reg(114u), Reg(119u), Reg(124u), Reg(118u), Reg(122u), Reg(117u), Reg(121u), Reg(120u), Reg(123u), Reg(116u)), RecursionImplWomPlonkElementLayout(Reg(35u), Reg4LayoutArray(Reg(36u), Reg(37u), Reg(38u), Reg(39u))), RecursionImplWomPlonkElementLayout(Reg(40u), Reg4LayoutArray(Reg(41u), Reg(42u), Reg(43u), Reg(44u))), RecursionImplWomPlonkElementLayout(Reg(45u), Reg4LayoutArray(Reg(46u), Reg(47u), Reg(48u), Reg(49u))))));
const kOutLayout: Reg32LayoutArray = Reg32LayoutArray(Reg(0u), Reg(1u), Reg(2u), Reg(3u), Reg(4u), Reg(5u), Reg(6u), Reg(7u), Reg(8u), Reg(9u), Reg(10u), Reg(11u), Reg(12u), Reg(13u), Reg(14u), Reg(15u), Reg(16u), Reg(17u), Reg(18u), Reg(19u), Reg(20u), Reg(21u), Reg(22u), Reg(23u), Reg(24u), Reg(25u), Reg(26u), Reg(27u), Reg(28u), Reg(29u), Reg(30u), Reg(31u));
