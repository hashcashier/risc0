struct NondetRegLayout {
  _super: Reg,
}
struct BoundLayout_NondetRegLayout {
  lyt: NondetRegLayout,
  buf: u32,
}
fn lookup_NondetRegLayout__super(b: BoundLayout_NondetRegLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt._super, b.buf);
}
alias NondetRegLayout7LayoutArray = array<NondetRegLayout, 7>;
struct BoundLayout_NondetRegLayout7LayoutArray {
  lyt: NondetRegLayout7LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout7LayoutArray(b: BoundLayout_NondetRegLayout7LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_7_Layout {
  _super: NondetRegLayout7LayoutArray,
}
struct BoundLayout_OneHot_7_Layout {
  lyt: OneHot_7_Layout,
  buf: u32,
}
fn lookup_OneHot_7_Layout__super(b: BoundLayout_OneHot_7_Layout) -> BoundLayout_NondetRegLayout7LayoutArray {
  return BoundLayout_NondetRegLayout7LayoutArray(b.lyt._super, b.buf);
}
struct NondetExtRegLayout {
  _super: Reg,
}
struct BoundLayout_NondetExtRegLayout {
  lyt: NondetExtRegLayout,
  buf: u32,
}
fn lookup_NondetExtRegLayout__super(b: BoundLayout_NondetExtRegLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt._super, b.buf);
}
struct BigIntAccumStateLayout {
  poly: NondetExtRegLayout,
  term: NondetExtRegLayout,
  total: NondetExtRegLayout,
}
struct BoundLayout_BigIntAccumStateLayout {
  lyt: BigIntAccumStateLayout,
  buf: u32,
}
fn lookup_BigIntAccumStateLayout_poly(b: BoundLayout_BigIntAccumStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.poly, b.buf);
}
fn lookup_BigIntAccumStateLayout_term(b: BoundLayout_BigIntAccumStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.term, b.buf);
}
fn lookup_BigIntAccumStateLayout_total(b: BoundLayout_BigIntAccumStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.total, b.buf);
}
struct BigIntPolyOpAddTotalLayout {
  _super: BigIntAccumStateLayout,
  tmp: NondetExtRegLayout,
}
struct BoundLayout_BigIntPolyOpAddTotalLayout {
  lyt: BigIntPolyOpAddTotalLayout,
  buf: u32,
}
fn lookup_BigIntPolyOpAddTotalLayout__super(b: BoundLayout_BigIntPolyOpAddTotalLayout) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntPolyOpAddTotalLayout_tmp(b: BoundLayout_BigIntPolyOpAddTotalLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.tmp, b.buf);
}
struct BigIntAccumStateLayout_0 {
  _super: BigIntAccumStateLayout,
  arm0: BigIntAccumStateLayout,
  arm1: BigIntAccumStateLayout,
  arm2: BigIntAccumStateLayout,
  arm3: BigIntPolyOpAddTotalLayout,
  arm4: BigIntAccumStateLayout,
  arm5: BigIntAccumStateLayout,
  arm6: BigIntAccumStateLayout,
}
struct BoundLayout_BigIntAccumStateLayout_0 {
  lyt: BigIntAccumStateLayout_0,
  buf: u32,
}
fn lookup_BigIntAccumStateLayout_0__super(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm0(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm0, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm1(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm1, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm2(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm2, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm3(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntPolyOpAddTotalLayout {
  return BoundLayout_BigIntPolyOpAddTotalLayout(b.lyt.arm3, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm4(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm4, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm5(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm5, b.buf);
}
fn lookup_BigIntAccumStateLayout_0_arm6(b: BoundLayout_BigIntAccumStateLayout_0) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.arm6, b.buf);
}
struct BigIntAccumLayout {
  polyOp: OneHot_7_Layout,
  state: BigIntAccumStateLayout,
  stateRedef: BigIntAccumStateLayout_0,
}
struct BoundLayout_BigIntAccumLayout {
  lyt: BigIntAccumLayout,
  buf: u32,
}
fn lookup_BigIntAccumLayout_polyOp(b: BoundLayout_BigIntAccumLayout) -> BoundLayout_OneHot_7_Layout {
  return BoundLayout_OneHot_7_Layout(b.lyt.polyOp, b.buf);
}
fn lookup_BigIntAccumLayout_state(b: BoundLayout_BigIntAccumLayout) -> BoundLayout_BigIntAccumStateLayout {
  return BoundLayout_BigIntAccumStateLayout(b.lyt.state, b.buf);
}
fn lookup_BigIntAccumLayout_stateRedef(b: BoundLayout_BigIntAccumLayout) -> BoundLayout_BigIntAccumStateLayout_0 {
  return BoundLayout_BigIntAccumStateLayout_0(b.lyt.stateRedef, b.buf);
}
struct AccumLayout {
  _0: BigIntAccumLayout,
}
struct BoundLayout_AccumLayout {
  lyt: AccumLayout,
  buf: u32,
}
fn lookup_AccumLayout__0(b: BoundLayout_AccumLayout) -> BoundLayout_BigIntAccumLayout {
  return BoundLayout_BigIntAccumLayout(b.lyt._0, b.buf);
}
alias NondetRegLayout8LayoutArray = array<NondetRegLayout, 8>;
struct BoundLayout_NondetRegLayout8LayoutArray {
  lyt: NondetRegLayout8LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout8LayoutArray(b: BoundLayout_NondetRegLayout8LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_8_Layout {
  _super: NondetRegLayout8LayoutArray,
}
struct BoundLayout_OneHot_8_Layout {
  lyt: OneHot_8_Layout,
  buf: u32,
}
fn lookup_OneHot_8_Layout__super(b: BoundLayout_OneHot_8_Layout) -> BoundLayout_NondetRegLayout8LayoutArray {
  return BoundLayout_NondetRegLayout8LayoutArray(b.lyt._super, b.buf);
}
struct InstInputLayout {
  minorOnehot: OneHot_8_Layout,
}
struct BoundLayout_InstInputLayout {
  lyt: InstInputLayout,
  buf: u32,
}
fn lookup_InstInputLayout_minorOnehot(b: BoundLayout_InstInputLayout) -> BoundLayout_OneHot_8_Layout {
  return BoundLayout_OneHot_8_Layout(b.lyt.minorOnehot, b.buf);
}
alias NondetRegLayout13LayoutArray = array<NondetRegLayout, 13>;
struct BoundLayout_NondetRegLayout13LayoutArray {
  lyt: NondetRegLayout13LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout13LayoutArray(b: BoundLayout_NondetRegLayout13LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_13_Layout {
  _super: NondetRegLayout13LayoutArray,
}
struct BoundLayout_OneHot_13_Layout {
  lyt: OneHot_13_Layout,
  buf: u32,
}
fn lookup_OneHot_13_Layout__super(b: BoundLayout_OneHot_13_Layout) -> BoundLayout_NondetRegLayout13LayoutArray {
  return BoundLayout_NondetRegLayout13LayoutArray(b.lyt._super, b.buf);
}
struct ArgU16Layout {
  count: NondetRegLayout,
  val: NondetRegLayout,
}
struct BoundLayout_ArgU16Layout {
  lyt: ArgU16Layout,
  buf: u32,
}
fn lookup_ArgU16Layout_count(b: BoundLayout_ArgU16Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_ArgU16Layout_val(b: BoundLayout_ArgU16Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.val, b.buf);
}
struct NondetU16RegLayout {
  arg: ArgU16Layout,
}
struct BoundLayout_NondetU16RegLayout {
  lyt: NondetU16RegLayout,
  buf: u32,
}
fn lookup_NondetU16RegLayout_arg(b: BoundLayout_NondetU16RegLayout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt.arg, b.buf);
}
struct NormalizeU32Layout {
  low16: NondetU16RegLayout,
  lowCarry: NondetRegLayout,
  high16: NondetU16RegLayout,
  highCarry: NondetRegLayout,
}
struct BoundLayout_NormalizeU32Layout {
  lyt: NormalizeU32Layout,
  buf: u32,
}
fn lookup_NormalizeU32Layout_low16(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low16, b.buf);
}
fn lookup_NormalizeU32Layout_lowCarry(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lowCarry, b.buf);
}
fn lookup_NormalizeU32Layout_high16(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.high16, b.buf);
}
fn lookup_NormalizeU32Layout_highCarry(b: BoundLayout_NormalizeU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.highCarry, b.buf);
}
struct MemoryArgLayout {
  count: NondetRegLayout,
  addr: NondetRegLayout,
  cycle: NondetRegLayout,
  dataLow: NondetRegLayout,
  dataHigh: NondetRegLayout,
}
struct BoundLayout_MemoryArgLayout {
  lyt: MemoryArgLayout,
  buf: u32,
}
fn lookup_MemoryArgLayout_count(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_MemoryArgLayout_addr(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.addr, b.buf);
}
fn lookup_MemoryArgLayout_cycle(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cycle, b.buf);
}
fn lookup_MemoryArgLayout_dataLow(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.dataLow, b.buf);
}
fn lookup_MemoryArgLayout_dataHigh(b: BoundLayout_MemoryArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.dataHigh, b.buf);
}
struct MemoryIOLayout {
  oldTxn: MemoryArgLayout,
  newTxn: MemoryArgLayout,
}
struct BoundLayout_MemoryIOLayout {
  lyt: MemoryIOLayout,
  buf: u32,
}
fn lookup_MemoryIOLayout_oldTxn(b: BoundLayout_MemoryIOLayout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt.oldTxn, b.buf);
}
fn lookup_MemoryIOLayout_newTxn(b: BoundLayout_MemoryIOLayout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt.newTxn, b.buf);
}
struct CycleArgLayout {
  count: NondetRegLayout,
  cycle: NondetRegLayout,
}
struct BoundLayout_CycleArgLayout {
  lyt: CycleArgLayout,
  buf: u32,
}
fn lookup_CycleArgLayout_count(b: BoundLayout_CycleArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_CycleArgLayout_cycle(b: BoundLayout_CycleArgLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cycle, b.buf);
}
struct IsCycleLayout {
  arg: CycleArgLayout,
}
struct BoundLayout_IsCycleLayout {
  lyt: IsCycleLayout,
  buf: u32,
}
fn lookup_IsCycleLayout_arg(b: BoundLayout_IsCycleLayout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt.arg, b.buf);
}
struct IsForwardLayout {
  _0: IsCycleLayout,
}
struct BoundLayout_IsForwardLayout {
  lyt: IsForwardLayout,
  buf: u32,
}
fn lookup_IsForwardLayout__0(b: BoundLayout_IsForwardLayout) -> BoundLayout_IsCycleLayout {
  return BoundLayout_IsCycleLayout(b.lyt._0, b.buf);
}
struct MemoryWriteLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryWriteLayout {
  lyt: MemoryWriteLayout,
  buf: u32,
}
fn lookup_MemoryWriteLayout_io(b: BoundLayout_MemoryWriteLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryWriteLayout__0(b: BoundLayout_MemoryWriteLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
struct IsZeroLayout {
  _super: NondetRegLayout,
  inv: NondetRegLayout,
}
struct BoundLayout_IsZeroLayout {
  lyt: IsZeroLayout,
  buf: u32,
}
fn lookup_IsZeroLayout__super(b: BoundLayout_IsZeroLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_IsZeroLayout_inv(b: BoundLayout_IsZeroLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.inv, b.buf);
}
struct WriteRdLayout {
  isRd0: IsZeroLayout,
  writeAddr: NondetRegLayout,
  _0: MemoryWriteLayout,
}
struct BoundLayout_WriteRdLayout {
  lyt: WriteRdLayout,
  buf: u32,
}
fn lookup_WriteRdLayout_isRd0(b: BoundLayout_WriteRdLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isRd0, b.buf);
}
fn lookup_WriteRdLayout_writeAddr(b: BoundLayout_WriteRdLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.writeAddr, b.buf);
}
fn lookup_WriteRdLayout__0(b: BoundLayout_WriteRdLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct FinalizeMiscLayout {
  writeData: NormalizeU32Layout,
  pcNorm: NormalizeU32Layout,
  _0: WriteRdLayout,
}
struct BoundLayout_FinalizeMiscLayout {
  lyt: FinalizeMiscLayout,
  buf: u32,
}
fn lookup_FinalizeMiscLayout_writeData(b: BoundLayout_FinalizeMiscLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.writeData, b.buf);
}
fn lookup_FinalizeMiscLayout_pcNorm(b: BoundLayout_FinalizeMiscLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcNorm, b.buf);
}
fn lookup_FinalizeMiscLayout__0(b: BoundLayout_FinalizeMiscLayout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._0, b.buf);
}
struct DoCycleTableLayout {
  arg1: CycleArgLayout,
  arg2: CycleArgLayout,
}
struct BoundLayout_DoCycleTableLayout {
  lyt: DoCycleTableLayout,
  buf: u32,
}
fn lookup_DoCycleTableLayout_arg1(b: BoundLayout_DoCycleTableLayout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt.arg1, b.buf);
}
fn lookup_DoCycleTableLayout_arg2(b: BoundLayout_DoCycleTableLayout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt.arg2, b.buf);
}
struct DecoderLayout {
  _f7_6: NondetRegLayout,
  _f7_45: NondetRegLayout,
  _f7_23: NondetRegLayout,
  _f7_01: NondetRegLayout,
  _rs2_34: NondetRegLayout,
  _rs2_12: NondetRegLayout,
  _rs2_0: NondetRegLayout,
  _rs1_34: NondetRegLayout,
  _rs1_12: NondetRegLayout,
  _rs1_0: NondetRegLayout,
  _f3_2: NondetRegLayout,
  _f3_01: NondetRegLayout,
  _rd_34: NondetRegLayout,
  _rd_12: NondetRegLayout,
  _rd_0: NondetRegLayout,
  opcode: NondetRegLayout,
}
struct BoundLayout_DecoderLayout {
  lyt: DecoderLayout,
  buf: u32,
}
fn lookup_DecoderLayout__f7_6(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_6, b.buf);
}
fn lookup_DecoderLayout__f7_45(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_45, b.buf);
}
fn lookup_DecoderLayout__f7_23(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_23, b.buf);
}
fn lookup_DecoderLayout__f7_01(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f7_01, b.buf);
}
fn lookup_DecoderLayout__rs2_34(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs2_34, b.buf);
}
fn lookup_DecoderLayout__rs2_12(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs2_12, b.buf);
}
fn lookup_DecoderLayout__rs2_0(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs2_0, b.buf);
}
fn lookup_DecoderLayout__rs1_34(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs1_34, b.buf);
}
fn lookup_DecoderLayout__rs1_12(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs1_12, b.buf);
}
fn lookup_DecoderLayout__rs1_0(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rs1_0, b.buf);
}
fn lookup_DecoderLayout__f3_2(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f3_2, b.buf);
}
fn lookup_DecoderLayout__f3_01(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._f3_01, b.buf);
}
fn lookup_DecoderLayout__rd_34(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rd_34, b.buf);
}
fn lookup_DecoderLayout__rd_12(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rd_12, b.buf);
}
fn lookup_DecoderLayout__rd_0(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._rd_0, b.buf);
}
fn lookup_DecoderLayout_opcode(b: BoundLayout_DecoderLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.opcode, b.buf);
}
struct AddrDecomposeLayout {
  low2: NondetRegLayout,
  upperDiff: NondetU16RegLayout,
  _0: IsZeroLayout,
  med14: NondetU16RegLayout,
}
struct BoundLayout_AddrDecomposeLayout {
  lyt: AddrDecomposeLayout,
  buf: u32,
}
fn lookup_AddrDecomposeLayout_low2(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low2, b.buf);
}
fn lookup_AddrDecomposeLayout_upperDiff(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.upperDiff, b.buf);
}
fn lookup_AddrDecomposeLayout__0(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
fn lookup_AddrDecomposeLayout_med14(b: BoundLayout_AddrDecomposeLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.med14, b.buf);
}
struct MemoryReadLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryReadLayout {
  lyt: MemoryReadLayout,
  buf: u32,
}
fn lookup_MemoryReadLayout_io(b: BoundLayout_MemoryReadLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryReadLayout__0(b: BoundLayout_MemoryReadLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
struct DecodeInstLayout {
  _super: DecoderLayout,
  pcAddr: AddrDecomposeLayout,
  loadInst: MemoryReadLayout,
}
struct BoundLayout_DecodeInstLayout {
  lyt: DecodeInstLayout,
  buf: u32,
}
fn lookup_DecodeInstLayout__super(b: BoundLayout_DecodeInstLayout) -> BoundLayout_DecoderLayout {
  return BoundLayout_DecoderLayout(b.lyt._super, b.buf);
}
fn lookup_DecodeInstLayout_pcAddr(b: BoundLayout_DecodeInstLayout) -> BoundLayout_AddrDecomposeLayout {
  return BoundLayout_AddrDecomposeLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_DecodeInstLayout_loadInst(b: BoundLayout_DecodeInstLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
alias MemoryArgLayout4LayoutArray = array<MemoryArgLayout, 4>;
struct BoundLayout_MemoryArgLayout4LayoutArray {
  lyt: MemoryArgLayout4LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout4LayoutArray(b: BoundLayout_MemoryArgLayout4LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout2LayoutArray = array<CycleArgLayout, 2>;
struct BoundLayout_CycleArgLayout2LayoutArray {
  lyt: CycleArgLayout2LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout2LayoutArray(b: BoundLayout_CycleArgLayout2LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_ReadSourceRegsSourceRegsLayout {
  memoryArg: MemoryArgLayout4LayoutArray,
  cycleArg: CycleArgLayout2LayoutArray,
}
struct BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout {
  lyt: _Arguments_ReadSourceRegsSourceRegsLayout,
  buf: u32,
}
fn lookup__Arguments_ReadSourceRegsSourceRegsLayout_memoryArg(b: BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout) -> BoundLayout_MemoryArgLayout4LayoutArray {
  return BoundLayout_MemoryArgLayout4LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ReadSourceRegsSourceRegsLayout_cycleArg(b: BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout) -> BoundLayout_CycleArgLayout2LayoutArray {
  return BoundLayout_CycleArgLayout2LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ReadRegLayout {
  _super: MemoryReadLayout,
  addr: NondetRegLayout,
}
struct BoundLayout_ReadRegLayout {
  lyt: ReadRegLayout,
  buf: u32,
}
fn lookup_ReadRegLayout__super(b: BoundLayout_ReadRegLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt._super, b.buf);
}
fn lookup_ReadRegLayout_addr(b: BoundLayout_ReadRegLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.addr, b.buf);
}
struct ReadSourceRegsSourceRegsArm0_SuperLayout {
  rboth: ReadRegLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout {
  lyt: ReadSourceRegsSourceRegsArm0_SuperLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsArm0_SuperLayout_rboth(b: BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt.rboth, b.buf);
}
struct ReadSourceRegsSourceRegsArm0Layout {
  _super: ReadSourceRegsSourceRegsArm0_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: CycleArgLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsArm0Layout {
  lyt: ReadSourceRegsSourceRegsArm0Layout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__super(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout {
  return BoundLayout_ReadSourceRegsSourceRegsArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__extra0(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__extra1(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm0Layout__extra2(b: BoundLayout_ReadSourceRegsSourceRegsArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra2, b.buf);
}
struct ReadSourceRegsSourceRegsArm1_SuperLayout {
  _0: ReadRegLayout,
  _1: ReadRegLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout {
  lyt: ReadSourceRegsSourceRegsArm1_SuperLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsArm1_SuperLayout__0(b: BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt._0, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsArm1_SuperLayout__1(b: BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt._1, b.buf);
}
struct ReadSourceRegsSourceRegsLayout {
  arm0: ReadSourceRegsSourceRegsArm0Layout,
  arm1: ReadSourceRegsSourceRegsArm1_SuperLayout,
}
struct BoundLayout_ReadSourceRegsSourceRegsLayout {
  lyt: ReadSourceRegsSourceRegsLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsSourceRegsLayout_arm0(b: BoundLayout_ReadSourceRegsSourceRegsLayout) -> BoundLayout_ReadSourceRegsSourceRegsArm0Layout {
  return BoundLayout_ReadSourceRegsSourceRegsArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ReadSourceRegsSourceRegsLayout_arm1(b: BoundLayout_ReadSourceRegsSourceRegsLayout) -> BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout {
  return BoundLayout_ReadSourceRegsSourceRegsArm1_SuperLayout(b.lyt.arm1, b.buf);
}
struct ReadSourceRegsLayout {
  isSameReg: NondetRegLayout,
  _arguments_ReadSourceRegsSourceRegs: _Arguments_ReadSourceRegsSourceRegsLayout,
  sourceRegs: ReadSourceRegsSourceRegsLayout,
  rs1Low: NondetRegLayout,
  rs1High: NondetRegLayout,
  rs2Low: NondetRegLayout,
  rs2High: NondetRegLayout,
}
struct BoundLayout_ReadSourceRegsLayout {
  lyt: ReadSourceRegsLayout,
  buf: u32,
}
fn lookup_ReadSourceRegsLayout_isSameReg(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isSameReg, b.buf);
}
fn lookup_ReadSourceRegsLayout__arguments_ReadSourceRegsSourceRegs(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout {
  return BoundLayout__Arguments_ReadSourceRegsSourceRegsLayout(b.lyt._arguments_ReadSourceRegsSourceRegs, b.buf);
}
fn lookup_ReadSourceRegsLayout_sourceRegs(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_ReadSourceRegsSourceRegsLayout {
  return BoundLayout_ReadSourceRegsSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs1Low(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs1Low, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs1High(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs1High, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs2Low(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs2Low, b.buf);
}
fn lookup_ReadSourceRegsLayout_rs2High(b: BoundLayout_ReadSourceRegsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.rs2High, b.buf);
}
struct MiscInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
}
struct BoundLayout_MiscInputLayout {
  lyt: MiscInputLayout,
  buf: u32,
}
fn lookup_MiscInputLayout_decoded(b: BoundLayout_MiscInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MiscInputLayout_sourceRegs(b: BoundLayout_MiscInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
alias ArgU16Layout5LayoutArray = array<ArgU16Layout, 5>;
struct BoundLayout_ArgU16Layout5LayoutArray {
  lyt: ArgU16Layout5LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout5LayoutArray(b: BoundLayout_ArgU16Layout5LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_Misc0MiscOutputLayout {
  argU16: ArgU16Layout5LayoutArray,
}
struct BoundLayout__Arguments_Misc0MiscOutputLayout {
  lyt: _Arguments_Misc0MiscOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Misc0MiscOutputLayout_argU16(b: BoundLayout__Arguments_Misc0MiscOutputLayout) -> BoundLayout_ArgU16Layout5LayoutArray {
  return BoundLayout_ArgU16Layout5LayoutArray(b.lyt.argU16, b.buf);
}
struct Misc0MiscOutputArm0Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm0Layout {
  lyt: Misc0MiscOutputArm0Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm0Layout__extra0(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra1(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra2(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra3(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm0Layout__extra4(b: BoundLayout_Misc0MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc0MiscOutputArm1Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm1Layout {
  lyt: Misc0MiscOutputArm1Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm1Layout__extra0(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra1(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra2(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra3(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm1Layout__extra4(b: BoundLayout_Misc0MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
alias NondetRegLayout16LayoutArray = array<NondetRegLayout, 16>;
struct BoundLayout_NondetRegLayout16LayoutArray {
  lyt: NondetRegLayout16LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout16LayoutArray(b: BoundLayout_NondetRegLayout16LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct ToBits_16_Layout {
  _super: NondetRegLayout16LayoutArray,
}
struct BoundLayout_ToBits_16_Layout {
  lyt: ToBits_16_Layout,
  buf: u32,
}
fn lookup_ToBits_16_Layout__super(b: BoundLayout_ToBits_16_Layout) -> BoundLayout_NondetRegLayout16LayoutArray {
  return BoundLayout_NondetRegLayout16LayoutArray(b.lyt._super, b.buf);
}
struct BitwiseAndU16Layout {
  bitsX: ToBits_16_Layout,
  bitsY: ToBits_16_Layout,
}
struct BoundLayout_BitwiseAndU16Layout {
  lyt: BitwiseAndU16Layout,
  buf: u32,
}
fn lookup_BitwiseAndU16Layout_bitsX(b: BoundLayout_BitwiseAndU16Layout) -> BoundLayout_ToBits_16_Layout {
  return BoundLayout_ToBits_16_Layout(b.lyt.bitsX, b.buf);
}
fn lookup_BitwiseAndU16Layout_bitsY(b: BoundLayout_BitwiseAndU16Layout) -> BoundLayout_ToBits_16_Layout {
  return BoundLayout_ToBits_16_Layout(b.lyt.bitsY, b.buf);
}
struct BitwiseAndLayout {
  _0: BitwiseAndU16Layout,
  _1: BitwiseAndU16Layout,
}
struct BoundLayout_BitwiseAndLayout {
  lyt: BitwiseAndLayout,
  buf: u32,
}
fn lookup_BitwiseAndLayout__0(b: BoundLayout_BitwiseAndLayout) -> BoundLayout_BitwiseAndU16Layout {
  return BoundLayout_BitwiseAndU16Layout(b.lyt._0, b.buf);
}
fn lookup_BitwiseAndLayout__1(b: BoundLayout_BitwiseAndLayout) -> BoundLayout_BitwiseAndU16Layout {
  return BoundLayout_BitwiseAndU16Layout(b.lyt._1, b.buf);
}
struct BitwiseXorLayout {
  andXy: BitwiseAndLayout,
}
struct BoundLayout_BitwiseXorLayout {
  lyt: BitwiseXorLayout,
  buf: u32,
}
fn lookup_BitwiseXorLayout_andXy(b: BoundLayout_BitwiseXorLayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt.andXy, b.buf);
}
struct OpXORLayout {
  _0: BitwiseXorLayout,
}
struct BoundLayout_OpXORLayout {
  lyt: OpXORLayout,
  buf: u32,
}
fn lookup_OpXORLayout__0(b: BoundLayout_OpXORLayout) -> BoundLayout_BitwiseXorLayout {
  return BoundLayout_BitwiseXorLayout(b.lyt._0, b.buf);
}
struct Misc0MiscOutputArm2Layout {
  _super: OpXORLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm2Layout {
  lyt: Misc0MiscOutputArm2Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm2Layout__super(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_OpXORLayout {
  return BoundLayout_OpXORLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra0(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra1(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra2(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra3(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm2Layout__extra4(b: BoundLayout_Misc0MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct BitwiseOrLayout {
  andXy: BitwiseAndLayout,
}
struct BoundLayout_BitwiseOrLayout {
  lyt: BitwiseOrLayout,
  buf: u32,
}
fn lookup_BitwiseOrLayout_andXy(b: BoundLayout_BitwiseOrLayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt.andXy, b.buf);
}
struct OpORLayout {
  _0: BitwiseOrLayout,
}
struct BoundLayout_OpORLayout {
  lyt: OpORLayout,
  buf: u32,
}
fn lookup_OpORLayout__0(b: BoundLayout_OpORLayout) -> BoundLayout_BitwiseOrLayout {
  return BoundLayout_BitwiseOrLayout(b.lyt._0, b.buf);
}
struct Misc0MiscOutputArm3Layout {
  _super: OpORLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm3Layout {
  lyt: Misc0MiscOutputArm3Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm3Layout__super(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_OpORLayout {
  return BoundLayout_OpORLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra0(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra1(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra2(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra3(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm3Layout__extra4(b: BoundLayout_Misc0MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpANDLayout {
  _0: BitwiseAndLayout,
}
struct BoundLayout_OpANDLayout {
  lyt: OpANDLayout,
  buf: u32,
}
fn lookup_OpANDLayout__0(b: BoundLayout_OpANDLayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt._0, b.buf);
}
struct Misc0MiscOutputArm4Layout {
  _super: OpANDLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm4Layout {
  lyt: Misc0MiscOutputArm4Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm4Layout__super(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_OpANDLayout {
  return BoundLayout_OpANDLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra0(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra1(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra2(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra3(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm4Layout__extra4(b: BoundLayout_Misc0MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct GetSignU32Layout {
  _super: NondetRegLayout,
  restTimesTwo: NondetU16RegLayout,
}
struct BoundLayout_GetSignU32Layout {
  lyt: GetSignU32Layout,
  buf: u32,
}
fn lookup_GetSignU32Layout__super(b: BoundLayout_GetSignU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_GetSignU32Layout_restTimesTwo(b: BoundLayout_GetSignU32Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.restTimesTwo, b.buf);
}
struct CmpLessThanLayout {
  diff: NormalizeU32Layout,
  s1: GetSignU32Layout,
  s2: GetSignU32Layout,
  s3: GetSignU32Layout,
  overflow: NondetRegLayout,
  isLessThan: NondetRegLayout,
}
struct BoundLayout_CmpLessThanLayout {
  lyt: CmpLessThanLayout,
  buf: u32,
}
fn lookup_CmpLessThanLayout_diff(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.diff, b.buf);
}
fn lookup_CmpLessThanLayout_s1(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_GetSignU32Layout {
  return BoundLayout_GetSignU32Layout(b.lyt.s1, b.buf);
}
fn lookup_CmpLessThanLayout_s2(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_GetSignU32Layout {
  return BoundLayout_GetSignU32Layout(b.lyt.s2, b.buf);
}
fn lookup_CmpLessThanLayout_s3(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_GetSignU32Layout {
  return BoundLayout_GetSignU32Layout(b.lyt.s3, b.buf);
}
fn lookup_CmpLessThanLayout_overflow(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.overflow, b.buf);
}
fn lookup_CmpLessThanLayout_isLessThan(b: BoundLayout_CmpLessThanLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isLessThan, b.buf);
}
struct OpSLTLayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpSLTLayout {
  lyt: OpSLTLayout,
  buf: u32,
}
fn lookup_OpSLTLayout_cmp(b: BoundLayout_OpSLTLayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct CmpLessThanUnsignedLayout {
  diff: NormalizeU32Layout,
}
struct BoundLayout_CmpLessThanUnsignedLayout {
  lyt: CmpLessThanUnsignedLayout,
  buf: u32,
}
fn lookup_CmpLessThanUnsignedLayout_diff(b: BoundLayout_CmpLessThanUnsignedLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.diff, b.buf);
}
struct OpSLTULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpSLTULayout {
  lyt: OpSLTULayout,
  buf: u32,
}
fn lookup_OpSLTULayout_cmp(b: BoundLayout_OpSLTULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc0MiscOutputArm6Layout {
  _super: OpSLTULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm6Layout {
  lyt: Misc0MiscOutputArm6Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm6Layout__super(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_OpSLTULayout {
  return BoundLayout_OpSLTULayout(b.lyt._super, b.buf);
}
fn lookup_Misc0MiscOutputArm6Layout__extra0(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm6Layout__extra1(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm6Layout__extra2(b: BoundLayout_Misc0MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct Misc0MiscOutputArm7Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc0MiscOutputArm7Layout {
  lyt: Misc0MiscOutputArm7Layout,
  buf: u32,
}
fn lookup_Misc0MiscOutputArm7Layout__extra0(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra1(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra2(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra3(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc0MiscOutputArm7Layout__extra4(b: BoundLayout_Misc0MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc0MiscOutputLayout {
  arm0: Misc0MiscOutputArm0Layout,
  arm1: Misc0MiscOutputArm1Layout,
  arm2: Misc0MiscOutputArm2Layout,
  arm3: Misc0MiscOutputArm3Layout,
  arm4: Misc0MiscOutputArm4Layout,
  arm5: OpSLTLayout,
  arm6: Misc0MiscOutputArm6Layout,
  arm7: Misc0MiscOutputArm7Layout,
}
struct BoundLayout_Misc0MiscOutputLayout {
  lyt: Misc0MiscOutputLayout,
  buf: u32,
}
fn lookup_Misc0MiscOutputLayout_arm0(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm0Layout {
  return BoundLayout_Misc0MiscOutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm1(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm1Layout {
  return BoundLayout_Misc0MiscOutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm2(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm2Layout {
  return BoundLayout_Misc0MiscOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm3(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm3Layout {
  return BoundLayout_Misc0MiscOutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm4(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm4Layout {
  return BoundLayout_Misc0MiscOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm5(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_OpSLTLayout {
  return BoundLayout_OpSLTLayout(b.lyt.arm5, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm6(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm6Layout {
  return BoundLayout_Misc0MiscOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Misc0MiscOutputLayout_arm7(b: BoundLayout_Misc0MiscOutputLayout) -> BoundLayout_Misc0MiscOutputArm7Layout {
  return BoundLayout_Misc0MiscOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Misc0Layout {
  _super: FinalizeMiscLayout,
  _0: DoCycleTableLayout,
  input: MiscInputLayout,
  _arguments_Misc0MiscOutput: _Arguments_Misc0MiscOutputLayout,
  miscOutput: Misc0MiscOutputLayout,
}
struct BoundLayout_Misc0Layout {
  lyt: Misc0Layout,
  buf: u32,
}
fn lookup_Misc0Layout__super(b: BoundLayout_Misc0Layout) -> BoundLayout_FinalizeMiscLayout {
  return BoundLayout_FinalizeMiscLayout(b.lyt._super, b.buf);
}
fn lookup_Misc0Layout__0(b: BoundLayout_Misc0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Misc0Layout_input(b: BoundLayout_Misc0Layout) -> BoundLayout_MiscInputLayout {
  return BoundLayout_MiscInputLayout(b.lyt.input, b.buf);
}
fn lookup_Misc0Layout__arguments_Misc0MiscOutput(b: BoundLayout_Misc0Layout) -> BoundLayout__Arguments_Misc0MiscOutputLayout {
  return BoundLayout__Arguments_Misc0MiscOutputLayout(b.lyt._arguments_Misc0MiscOutput, b.buf);
}
fn lookup_Misc0Layout_miscOutput(b: BoundLayout_Misc0Layout) -> BoundLayout_Misc0MiscOutputLayout {
  return BoundLayout_Misc0MiscOutputLayout(b.lyt.miscOutput, b.buf);
}
struct _Arguments_Misc1MiscOutputLayout {
  argU16: ArgU16Layout5LayoutArray,
}
struct BoundLayout__Arguments_Misc1MiscOutputLayout {
  lyt: _Arguments_Misc1MiscOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Misc1MiscOutputLayout_argU16(b: BoundLayout__Arguments_Misc1MiscOutputLayout) -> BoundLayout_ArgU16Layout5LayoutArray {
  return BoundLayout_ArgU16Layout5LayoutArray(b.lyt.argU16, b.buf);
}
struct OpXORILayout {
  _0: BitwiseXorLayout,
}
struct BoundLayout_OpXORILayout {
  lyt: OpXORILayout,
  buf: u32,
}
fn lookup_OpXORILayout__0(b: BoundLayout_OpXORILayout) -> BoundLayout_BitwiseXorLayout {
  return BoundLayout_BitwiseXorLayout(b.lyt._0, b.buf);
}
struct Misc1MiscOutputArm0Layout {
  _super: OpXORILayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm0Layout {
  lyt: Misc1MiscOutputArm0Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm0Layout__super(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_OpXORILayout {
  return BoundLayout_OpXORILayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra0(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra1(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra2(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra3(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm0Layout__extra4(b: BoundLayout_Misc1MiscOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpORILayout {
  _0: BitwiseOrLayout,
}
struct BoundLayout_OpORILayout {
  lyt: OpORILayout,
  buf: u32,
}
fn lookup_OpORILayout__0(b: BoundLayout_OpORILayout) -> BoundLayout_BitwiseOrLayout {
  return BoundLayout_BitwiseOrLayout(b.lyt._0, b.buf);
}
struct Misc1MiscOutputArm1Layout {
  _super: OpORILayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm1Layout {
  lyt: Misc1MiscOutputArm1Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm1Layout__super(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_OpORILayout {
  return BoundLayout_OpORILayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra0(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra1(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra2(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra3(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm1Layout__extra4(b: BoundLayout_Misc1MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpANDILayout {
  _0: BitwiseAndLayout,
}
struct BoundLayout_OpANDILayout {
  lyt: OpANDILayout,
  buf: u32,
}
fn lookup_OpANDILayout__0(b: BoundLayout_OpANDILayout) -> BoundLayout_BitwiseAndLayout {
  return BoundLayout_BitwiseAndLayout(b.lyt._0, b.buf);
}
struct Misc1MiscOutputArm2Layout {
  _super: OpANDILayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm2Layout {
  lyt: Misc1MiscOutputArm2Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm2Layout__super(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_OpANDILayout {
  return BoundLayout_OpANDILayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra0(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra1(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra2(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra3(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm2Layout__extra4(b: BoundLayout_Misc1MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpSLTILayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpSLTILayout {
  lyt: OpSLTILayout,
  buf: u32,
}
fn lookup_OpSLTILayout_cmp(b: BoundLayout_OpSLTILayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct OpSLTIULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpSLTIULayout {
  lyt: OpSLTIULayout,
  buf: u32,
}
fn lookup_OpSLTIULayout_cmp(b: BoundLayout_OpSLTIULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputArm4Layout {
  _super: OpSLTIULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm4Layout {
  lyt: Misc1MiscOutputArm4Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm4Layout__super(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_OpSLTIULayout {
  return BoundLayout_OpSLTIULayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm4Layout__extra0(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm4Layout__extra1(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm4Layout__extra2(b: BoundLayout_Misc1MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct CmpEqualLayout {
  lowSame: IsZeroLayout,
  highSame: IsZeroLayout,
  isEqual: NondetRegLayout,
}
struct BoundLayout_CmpEqualLayout {
  lyt: CmpEqualLayout,
  buf: u32,
}
fn lookup_CmpEqualLayout_lowSame(b: BoundLayout_CmpEqualLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lowSame, b.buf);
}
fn lookup_CmpEqualLayout_highSame(b: BoundLayout_CmpEqualLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.highSame, b.buf);
}
fn lookup_CmpEqualLayout_isEqual(b: BoundLayout_CmpEqualLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isEqual, b.buf);
}
struct OpBEQLayout {
  cmp: CmpEqualLayout,
}
struct BoundLayout_OpBEQLayout {
  lyt: OpBEQLayout,
  buf: u32,
}
fn lookup_OpBEQLayout_cmp(b: BoundLayout_OpBEQLayout) -> BoundLayout_CmpEqualLayout {
  return BoundLayout_CmpEqualLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputArm5Layout {
  _super: OpBEQLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm5Layout {
  lyt: Misc1MiscOutputArm5Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm5Layout__super(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_OpBEQLayout {
  return BoundLayout_OpBEQLayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra0(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra1(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra2(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra3(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm5Layout__extra4(b: BoundLayout_Misc1MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpBNELayout {
  cmp: CmpEqualLayout,
}
struct BoundLayout_OpBNELayout {
  lyt: OpBNELayout,
  buf: u32,
}
fn lookup_OpBNELayout_cmp(b: BoundLayout_OpBNELayout) -> BoundLayout_CmpEqualLayout {
  return BoundLayout_CmpEqualLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputArm6Layout {
  _super: OpBNELayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc1MiscOutputArm6Layout {
  lyt: Misc1MiscOutputArm6Layout,
  buf: u32,
}
fn lookup_Misc1MiscOutputArm6Layout__super(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_OpBNELayout {
  return BoundLayout_OpBNELayout(b.lyt._super, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra0(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra1(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra2(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra3(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc1MiscOutputArm6Layout__extra4(b: BoundLayout_Misc1MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpBLTLayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpBLTLayout {
  lyt: OpBLTLayout,
  buf: u32,
}
fn lookup_OpBLTLayout_cmp(b: BoundLayout_OpBLTLayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct Misc1MiscOutputLayout {
  arm0: Misc1MiscOutputArm0Layout,
  arm1: Misc1MiscOutputArm1Layout,
  arm2: Misc1MiscOutputArm2Layout,
  arm3: OpSLTILayout,
  arm4: Misc1MiscOutputArm4Layout,
  arm5: Misc1MiscOutputArm5Layout,
  arm6: Misc1MiscOutputArm6Layout,
  arm7: OpBLTLayout,
}
struct BoundLayout_Misc1MiscOutputLayout {
  lyt: Misc1MiscOutputLayout,
  buf: u32,
}
fn lookup_Misc1MiscOutputLayout_arm0(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm0Layout {
  return BoundLayout_Misc1MiscOutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm1(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm1Layout {
  return BoundLayout_Misc1MiscOutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm2(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm2Layout {
  return BoundLayout_Misc1MiscOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm3(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_OpSLTILayout {
  return BoundLayout_OpSLTILayout(b.lyt.arm3, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm4(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm4Layout {
  return BoundLayout_Misc1MiscOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm5(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm5Layout {
  return BoundLayout_Misc1MiscOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm6(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_Misc1MiscOutputArm6Layout {
  return BoundLayout_Misc1MiscOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Misc1MiscOutputLayout_arm7(b: BoundLayout_Misc1MiscOutputLayout) -> BoundLayout_OpBLTLayout {
  return BoundLayout_OpBLTLayout(b.lyt.arm7, b.buf);
}
struct Misc1Layout {
  _super: FinalizeMiscLayout,
  _0: DoCycleTableLayout,
  input: MiscInputLayout,
  _arguments_Misc1MiscOutput: _Arguments_Misc1MiscOutputLayout,
  miscOutput: Misc1MiscOutputLayout,
}
struct BoundLayout_Misc1Layout {
  lyt: Misc1Layout,
  buf: u32,
}
fn lookup_Misc1Layout__super(b: BoundLayout_Misc1Layout) -> BoundLayout_FinalizeMiscLayout {
  return BoundLayout_FinalizeMiscLayout(b.lyt._super, b.buf);
}
fn lookup_Misc1Layout__0(b: BoundLayout_Misc1Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Misc1Layout_input(b: BoundLayout_Misc1Layout) -> BoundLayout_MiscInputLayout {
  return BoundLayout_MiscInputLayout(b.lyt.input, b.buf);
}
fn lookup_Misc1Layout__arguments_Misc1MiscOutput(b: BoundLayout_Misc1Layout) -> BoundLayout__Arguments_Misc1MiscOutputLayout {
  return BoundLayout__Arguments_Misc1MiscOutputLayout(b.lyt._arguments_Misc1MiscOutput, b.buf);
}
fn lookup_Misc1Layout_miscOutput(b: BoundLayout_Misc1Layout) -> BoundLayout_Misc1MiscOutputLayout {
  return BoundLayout_Misc1MiscOutputLayout(b.lyt.miscOutput, b.buf);
}
struct _Arguments_Misc2MiscOutputLayout {
  argU16: ArgU16Layout5LayoutArray,
}
struct BoundLayout__Arguments_Misc2MiscOutputLayout {
  lyt: _Arguments_Misc2MiscOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Misc2MiscOutputLayout_argU16(b: BoundLayout__Arguments_Misc2MiscOutputLayout) -> BoundLayout_ArgU16Layout5LayoutArray {
  return BoundLayout_ArgU16Layout5LayoutArray(b.lyt.argU16, b.buf);
}
struct OpBGELayout {
  cmp: CmpLessThanLayout,
}
struct BoundLayout_OpBGELayout {
  lyt: OpBGELayout,
  buf: u32,
}
fn lookup_OpBGELayout_cmp(b: BoundLayout_OpBGELayout) -> BoundLayout_CmpLessThanLayout {
  return BoundLayout_CmpLessThanLayout(b.lyt.cmp, b.buf);
}
struct OpBLTULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpBLTULayout {
  lyt: OpBLTULayout,
  buf: u32,
}
fn lookup_OpBLTULayout_cmp(b: BoundLayout_OpBLTULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc2MiscOutputArm1Layout {
  _super: OpBLTULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm1Layout {
  lyt: Misc2MiscOutputArm1Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm1Layout__super(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_OpBLTULayout {
  return BoundLayout_OpBLTULayout(b.lyt._super, b.buf);
}
fn lookup_Misc2MiscOutputArm1Layout__extra0(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm1Layout__extra1(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm1Layout__extra2(b: BoundLayout_Misc2MiscOutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct OpBGEULayout {
  cmp: CmpLessThanUnsignedLayout,
}
struct BoundLayout_OpBGEULayout {
  lyt: OpBGEULayout,
  buf: u32,
}
fn lookup_OpBGEULayout_cmp(b: BoundLayout_OpBGEULayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.cmp, b.buf);
}
struct Misc2MiscOutputArm2Layout {
  _super: OpBGEULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm2Layout {
  lyt: Misc2MiscOutputArm2Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm2Layout__super(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_OpBGEULayout {
  return BoundLayout_OpBGEULayout(b.lyt._super, b.buf);
}
fn lookup_Misc2MiscOutputArm2Layout__extra0(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm2Layout__extra1(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm2Layout__extra2(b: BoundLayout_Misc2MiscOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
struct Misc2MiscOutputArm3Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm3Layout {
  lyt: Misc2MiscOutputArm3Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm3Layout__extra0(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra1(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra2(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra3(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm3Layout__extra4(b: BoundLayout_Misc2MiscOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct OpJALRLayout {
  lsb: NondetRegLayout,
  half: NondetU16RegLayout,
}
struct BoundLayout_OpJALRLayout {
  lyt: OpJALRLayout,
  buf: u32,
}
fn lookup_OpJALRLayout_lsb(b: BoundLayout_OpJALRLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lsb, b.buf);
}
fn lookup_OpJALRLayout_half(b: BoundLayout_OpJALRLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.half, b.buf);
}
struct Misc2MiscOutputArm4Layout {
  _super: OpJALRLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm4Layout {
  lyt: Misc2MiscOutputArm4Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm4Layout__super(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_OpJALRLayout {
  return BoundLayout_OpJALRLayout(b.lyt._super, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra0(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra1(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra2(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm4Layout__extra3(b: BoundLayout_Misc2MiscOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Misc2MiscOutputArm5Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm5Layout {
  lyt: Misc2MiscOutputArm5Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm5Layout__extra0(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra1(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra2(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra3(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm5Layout__extra4(b: BoundLayout_Misc2MiscOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc2MiscOutputArm6Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm6Layout {
  lyt: Misc2MiscOutputArm6Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm6Layout__extra0(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra1(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra2(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra3(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm6Layout__extra4(b: BoundLayout_Misc2MiscOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc2MiscOutputArm7Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
}
struct BoundLayout_Misc2MiscOutputArm7Layout {
  lyt: Misc2MiscOutputArm7Layout,
  buf: u32,
}
fn lookup_Misc2MiscOutputArm7Layout__extra0(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra1(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra2(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra3(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Misc2MiscOutputArm7Layout__extra4(b: BoundLayout_Misc2MiscOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
struct Misc2MiscOutputLayout {
  arm0: OpBGELayout,
  arm1: Misc2MiscOutputArm1Layout,
  arm2: Misc2MiscOutputArm2Layout,
  arm3: Misc2MiscOutputArm3Layout,
  arm4: Misc2MiscOutputArm4Layout,
  arm5: Misc2MiscOutputArm5Layout,
  arm6: Misc2MiscOutputArm6Layout,
  arm7: Misc2MiscOutputArm7Layout,
}
struct BoundLayout_Misc2MiscOutputLayout {
  lyt: Misc2MiscOutputLayout,
  buf: u32,
}
fn lookup_Misc2MiscOutputLayout_arm0(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_OpBGELayout {
  return BoundLayout_OpBGELayout(b.lyt.arm0, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm1(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm1Layout {
  return BoundLayout_Misc2MiscOutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm2(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm2Layout {
  return BoundLayout_Misc2MiscOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm3(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm3Layout {
  return BoundLayout_Misc2MiscOutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm4(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm4Layout {
  return BoundLayout_Misc2MiscOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm5(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm5Layout {
  return BoundLayout_Misc2MiscOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm6(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm6Layout {
  return BoundLayout_Misc2MiscOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Misc2MiscOutputLayout_arm7(b: BoundLayout_Misc2MiscOutputLayout) -> BoundLayout_Misc2MiscOutputArm7Layout {
  return BoundLayout_Misc2MiscOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Misc2Layout {
  _super: FinalizeMiscLayout,
  _0: DoCycleTableLayout,
  input: MiscInputLayout,
  _arguments_Misc2MiscOutput: _Arguments_Misc2MiscOutputLayout,
  miscOutput: Misc2MiscOutputLayout,
}
struct BoundLayout_Misc2Layout {
  lyt: Misc2Layout,
  buf: u32,
}
fn lookup_Misc2Layout__super(b: BoundLayout_Misc2Layout) -> BoundLayout_FinalizeMiscLayout {
  return BoundLayout_FinalizeMiscLayout(b.lyt._super, b.buf);
}
fn lookup_Misc2Layout__0(b: BoundLayout_Misc2Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Misc2Layout_input(b: BoundLayout_Misc2Layout) -> BoundLayout_MiscInputLayout {
  return BoundLayout_MiscInputLayout(b.lyt.input, b.buf);
}
fn lookup_Misc2Layout__arguments_Misc2MiscOutput(b: BoundLayout_Misc2Layout) -> BoundLayout__Arguments_Misc2MiscOutputLayout {
  return BoundLayout__Arguments_Misc2MiscOutputLayout(b.lyt._arguments_Misc2MiscOutput, b.buf);
}
fn lookup_Misc2Layout_miscOutput(b: BoundLayout_Misc2Layout) -> BoundLayout_Misc2MiscOutputLayout {
  return BoundLayout_Misc2MiscOutputLayout(b.lyt.miscOutput, b.buf);
}
struct MulInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
}
struct BoundLayout_MulInputLayout {
  lyt: MulInputLayout,
  buf: u32,
}
fn lookup_MulInputLayout_decoded(b: BoundLayout_MulInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MulInputLayout_sourceRegs(b: BoundLayout_MulInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
alias ArgU16Layout6LayoutArray = array<ArgU16Layout, 6>;
struct BoundLayout_ArgU16Layout6LayoutArray {
  lyt: ArgU16Layout6LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout6LayoutArray(b: BoundLayout_ArgU16Layout6LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct ArgU8Layout {
  count: NondetRegLayout,
  val: NondetRegLayout,
}
struct BoundLayout_ArgU8Layout {
  lyt: ArgU8Layout,
  buf: u32,
}
fn lookup_ArgU8Layout_count(b: BoundLayout_ArgU8Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_ArgU8Layout_val(b: BoundLayout_ArgU8Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.val, b.buf);
}
alias ArgU8Layout13LayoutArray = array<ArgU8Layout, 13>;
struct BoundLayout_ArgU8Layout13LayoutArray {
  lyt: ArgU8Layout13LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout13LayoutArray(b: BoundLayout_ArgU8Layout13LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_Mul0MulOutputLayout {
  argU16: ArgU16Layout6LayoutArray,
  argU8: ArgU8Layout13LayoutArray,
}
struct BoundLayout__Arguments_Mul0MulOutputLayout {
  lyt: _Arguments_Mul0MulOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Mul0MulOutputLayout_argU16(b: BoundLayout__Arguments_Mul0MulOutputLayout) -> BoundLayout_ArgU16Layout6LayoutArray {
  return BoundLayout_ArgU16Layout6LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Mul0MulOutputLayout_argU8(b: BoundLayout__Arguments_Mul0MulOutputLayout) -> BoundLayout_ArgU8Layout13LayoutArray {
  return BoundLayout_ArgU8Layout13LayoutArray(b.lyt.argU8, b.buf);
}
alias NondetRegLayout5LayoutArray = array<NondetRegLayout, 5>;
struct BoundLayout_NondetRegLayout5LayoutArray {
  lyt: NondetRegLayout5LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout5LayoutArray(b: BoundLayout_NondetRegLayout5LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct ToBits_5_Layout {
  _super: NondetRegLayout5LayoutArray,
}
struct BoundLayout_ToBits_5_Layout {
  lyt: ToBits_5_Layout,
  buf: u32,
}
fn lookup_ToBits_5_Layout__super(b: BoundLayout_ToBits_5_Layout) -> BoundLayout_NondetRegLayout5LayoutArray {
  return BoundLayout_NondetRegLayout5LayoutArray(b.lyt._super, b.buf);
}
struct DynPo2Layout {
  low5: ToBits_5_Layout,
  checkU16: NondetU16RegLayout,
  b3: NondetRegLayout,
  low: NondetRegLayout,
  high: NondetRegLayout,
}
struct BoundLayout_DynPo2Layout {
  lyt: DynPo2Layout,
  buf: u32,
}
fn lookup_DynPo2Layout_low5(b: BoundLayout_DynPo2Layout) -> BoundLayout_ToBits_5_Layout {
  return BoundLayout_ToBits_5_Layout(b.lyt.low5, b.buf);
}
fn lookup_DynPo2Layout_checkU16(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.checkU16, b.buf);
}
fn lookup_DynPo2Layout_b3(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.b3, b.buf);
}
fn lookup_DynPo2Layout_low(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low, b.buf);
}
fn lookup_DynPo2Layout_high(b: BoundLayout_DynPo2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.high, b.buf);
}
struct NondetU8RegLayout {
  arg: ArgU8Layout,
}
struct BoundLayout_NondetU8RegLayout {
  lyt: NondetU8RegLayout,
  buf: u32,
}
fn lookup_NondetU8RegLayout_arg(b: BoundLayout_NondetU8RegLayout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt.arg, b.buf);
}
struct ExpandU32Layout {
  b0: NondetU8RegLayout,
  b1: NondetU8RegLayout,
  b2: NondetU8RegLayout,
  b3: NondetU8RegLayout,
  b3Top7times2: NondetU8RegLayout,
  topBit: NondetRegLayout,
}
struct BoundLayout_ExpandU32Layout {
  lyt: ExpandU32Layout,
  buf: u32,
}
fn lookup_ExpandU32Layout_b0(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b0, b.buf);
}
fn lookup_ExpandU32Layout_b1(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b1, b.buf);
}
fn lookup_ExpandU32Layout_b2(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b2, b.buf);
}
fn lookup_ExpandU32Layout_b3(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b3, b.buf);
}
fn lookup_ExpandU32Layout_b3Top7times2(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.b3Top7times2, b.buf);
}
fn lookup_ExpandU32Layout_topBit(b: BoundLayout_ExpandU32Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.topBit, b.buf);
}
struct NondetFakeTwitRegLayout {
  reg0: NondetRegLayout,
  reg1: NondetRegLayout,
}
struct BoundLayout_NondetFakeTwitRegLayout {
  lyt: NondetFakeTwitRegLayout,
  buf: u32,
}
fn lookup_NondetFakeTwitRegLayout_reg0(b: BoundLayout_NondetFakeTwitRegLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.reg0, b.buf);
}
fn lookup_NondetFakeTwitRegLayout_reg1(b: BoundLayout_NondetFakeTwitRegLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.reg1, b.buf);
}
struct SplitTotalLayout {
  out_: NondetU16RegLayout,
  carryByte: NondetU8RegLayout,
  carryExtra: NondetFakeTwitRegLayout,
}
struct BoundLayout_SplitTotalLayout {
  lyt: SplitTotalLayout,
  buf: u32,
}
fn lookup_SplitTotalLayout_out_(b: BoundLayout_SplitTotalLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.out_, b.buf);
}
fn lookup_SplitTotalLayout_carryByte(b: BoundLayout_SplitTotalLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.carryByte, b.buf);
}
fn lookup_SplitTotalLayout_carryExtra(b: BoundLayout_SplitTotalLayout) -> BoundLayout_NondetFakeTwitRegLayout {
  return BoundLayout_NondetFakeTwitRegLayout(b.lyt.carryExtra, b.buf);
}
struct MultiplyAccumulateLayout {
  ax: ExpandU32Layout,
  bx: ExpandU32Layout,
  cSign: NondetRegLayout,
  cRestTimes2: NondetU16RegLayout,
  s0: SplitTotalLayout,
  s1: SplitTotalLayout,
  s2: SplitTotalLayout,
  s3Out: NondetU16RegLayout,
  s3Carry: NondetFakeTwitRegLayout,
}
struct BoundLayout_MultiplyAccumulateLayout {
  lyt: MultiplyAccumulateLayout,
  buf: u32,
}
fn lookup_MultiplyAccumulateLayout_ax(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_ExpandU32Layout {
  return BoundLayout_ExpandU32Layout(b.lyt.ax, b.buf);
}
fn lookup_MultiplyAccumulateLayout_bx(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_ExpandU32Layout {
  return BoundLayout_ExpandU32Layout(b.lyt.bx, b.buf);
}
fn lookup_MultiplyAccumulateLayout_cSign(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cSign, b.buf);
}
fn lookup_MultiplyAccumulateLayout_cRestTimes2(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.cRestTimes2, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s0(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_SplitTotalLayout {
  return BoundLayout_SplitTotalLayout(b.lyt.s0, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s1(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_SplitTotalLayout {
  return BoundLayout_SplitTotalLayout(b.lyt.s1, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s2(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_SplitTotalLayout {
  return BoundLayout_SplitTotalLayout(b.lyt.s2, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s3Out(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.s3Out, b.buf);
}
fn lookup_MultiplyAccumulateLayout_s3Carry(b: BoundLayout_MultiplyAccumulateLayout) -> BoundLayout_NondetFakeTwitRegLayout {
  return BoundLayout_NondetFakeTwitRegLayout(b.lyt.s3Carry, b.buf);
}
struct DoMulLayout {
  mul: MultiplyAccumulateLayout,
}
struct BoundLayout_DoMulLayout {
  lyt: DoMulLayout,
  buf: u32,
}
fn lookup_DoMulLayout_mul(b: BoundLayout_DoMulLayout) -> BoundLayout_MultiplyAccumulateLayout {
  return BoundLayout_MultiplyAccumulateLayout(b.lyt.mul, b.buf);
}
struct OpSLLLayout {
  shiftMul: DynPo2Layout,
  _0: DoMulLayout,
}
struct BoundLayout_OpSLLLayout {
  lyt: OpSLLLayout,
  buf: u32,
}
fn lookup_OpSLLLayout_shiftMul(b: BoundLayout_OpSLLLayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSLLLayout__0(b: BoundLayout_OpSLLLayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct OpSLLILayout {
  shiftMul: DynPo2Layout,
  _0: DoMulLayout,
}
struct BoundLayout_OpSLLILayout {
  lyt: OpSLLILayout,
  buf: u32,
}
fn lookup_OpSLLILayout_shiftMul(b: BoundLayout_OpSLLILayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSLLILayout__0(b: BoundLayout_OpSLLILayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct OpMULLayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULLayout {
  lyt: OpMULLayout,
  buf: u32,
}
fn lookup_OpMULLayout__0(b: BoundLayout_OpMULLayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm2Layout {
  _super: OpMULLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm2Layout {
  lyt: Mul0MulOutputArm2Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm2Layout__super(b: BoundLayout_Mul0MulOutputArm2Layout) -> BoundLayout_OpMULLayout {
  return BoundLayout_OpMULLayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm2Layout__extra0(b: BoundLayout_Mul0MulOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpMULHLayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULHLayout {
  lyt: OpMULHLayout,
  buf: u32,
}
fn lookup_OpMULHLayout__0(b: BoundLayout_OpMULHLayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm3Layout {
  _super: OpMULHLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm3Layout {
  lyt: Mul0MulOutputArm3Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm3Layout__super(b: BoundLayout_Mul0MulOutputArm3Layout) -> BoundLayout_OpMULHLayout {
  return BoundLayout_OpMULHLayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm3Layout__extra0(b: BoundLayout_Mul0MulOutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpMULHSULayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULHSULayout {
  lyt: OpMULHSULayout,
  buf: u32,
}
fn lookup_OpMULHSULayout__0(b: BoundLayout_OpMULHSULayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm4Layout {
  _super: OpMULHSULayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm4Layout {
  lyt: Mul0MulOutputArm4Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm4Layout__super(b: BoundLayout_Mul0MulOutputArm4Layout) -> BoundLayout_OpMULHSULayout {
  return BoundLayout_OpMULHSULayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm4Layout__extra0(b: BoundLayout_Mul0MulOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpMULHULayout {
  _0: DoMulLayout,
}
struct BoundLayout_OpMULHULayout {
  lyt: OpMULHULayout,
  buf: u32,
}
fn lookup_OpMULHULayout__0(b: BoundLayout_OpMULHULayout) -> BoundLayout_DoMulLayout {
  return BoundLayout_DoMulLayout(b.lyt._0, b.buf);
}
struct Mul0MulOutputArm5Layout {
  _super: OpMULHULayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mul0MulOutputArm5Layout {
  lyt: Mul0MulOutputArm5Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm5Layout__super(b: BoundLayout_Mul0MulOutputArm5Layout) -> BoundLayout_OpMULHULayout {
  return BoundLayout_OpMULHULayout(b.lyt._super, b.buf);
}
fn lookup_Mul0MulOutputArm5Layout__extra0(b: BoundLayout_Mul0MulOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct Mul0MulOutputArm6Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU8Layout,
  _extra7: ArgU8Layout,
  _extra8: ArgU8Layout,
  _extra9: ArgU8Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
}
struct BoundLayout_Mul0MulOutputArm6Layout {
  lyt: Mul0MulOutputArm6Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm6Layout__extra0(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra1(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra2(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra3(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra4(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra5(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra6(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra6, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra7(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra7, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra8(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra8, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra9(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra9, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra10(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra11(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra12(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra13(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra14(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra15(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra16(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra17(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Mul0MulOutputArm6Layout__extra18(b: BoundLayout_Mul0MulOutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
struct Mul0MulOutputArm7Layout {
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU8Layout,
  _extra7: ArgU8Layout,
  _extra8: ArgU8Layout,
  _extra9: ArgU8Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
}
struct BoundLayout_Mul0MulOutputArm7Layout {
  lyt: Mul0MulOutputArm7Layout,
  buf: u32,
}
fn lookup_Mul0MulOutputArm7Layout__extra0(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra1(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra2(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra3(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra4(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra5(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra6(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra6, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra7(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra7, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra8(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra8, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra9(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra9, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra10(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra11(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra12(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra13(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra14(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra15(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra16(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra17(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Mul0MulOutputArm7Layout__extra18(b: BoundLayout_Mul0MulOutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
struct Mul0MulOutputLayout {
  arm0: OpSLLLayout,
  arm1: OpSLLILayout,
  arm2: Mul0MulOutputArm2Layout,
  arm3: Mul0MulOutputArm3Layout,
  arm4: Mul0MulOutputArm4Layout,
  arm5: Mul0MulOutputArm5Layout,
  arm6: Mul0MulOutputArm6Layout,
  arm7: Mul0MulOutputArm7Layout,
}
struct BoundLayout_Mul0MulOutputLayout {
  lyt: Mul0MulOutputLayout,
  buf: u32,
}
fn lookup_Mul0MulOutputLayout_arm0(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_OpSLLLayout {
  return BoundLayout_OpSLLLayout(b.lyt.arm0, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm1(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_OpSLLILayout {
  return BoundLayout_OpSLLILayout(b.lyt.arm1, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm2(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm2Layout {
  return BoundLayout_Mul0MulOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm3(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm3Layout {
  return BoundLayout_Mul0MulOutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm4(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm4Layout {
  return BoundLayout_Mul0MulOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm5(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm5Layout {
  return BoundLayout_Mul0MulOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm6(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm6Layout {
  return BoundLayout_Mul0MulOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Mul0MulOutputLayout_arm7(b: BoundLayout_Mul0MulOutputLayout) -> BoundLayout_Mul0MulOutputArm7Layout {
  return BoundLayout_Mul0MulOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Mul0Layout {
  _0: DoCycleTableLayout,
  input: MulInputLayout,
  _arguments_Mul0MulOutput: _Arguments_Mul0MulOutputLayout,
  mulOutput: Mul0MulOutputLayout,
  _1: WriteRdLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Mul0Layout {
  lyt: Mul0Layout,
  buf: u32,
}
fn lookup_Mul0Layout__0(b: BoundLayout_Mul0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Mul0Layout_input(b: BoundLayout_Mul0Layout) -> BoundLayout_MulInputLayout {
  return BoundLayout_MulInputLayout(b.lyt.input, b.buf);
}
fn lookup_Mul0Layout__arguments_Mul0MulOutput(b: BoundLayout_Mul0Layout) -> BoundLayout__Arguments_Mul0MulOutputLayout {
  return BoundLayout__Arguments_Mul0MulOutputLayout(b.lyt._arguments_Mul0MulOutput, b.buf);
}
fn lookup_Mul0Layout_mulOutput(b: BoundLayout_Mul0Layout) -> BoundLayout_Mul0MulOutputLayout {
  return BoundLayout_Mul0MulOutputLayout(b.lyt.mulOutput, b.buf);
}
fn lookup_Mul0Layout__1(b: BoundLayout_Mul0Layout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._1, b.buf);
}
fn lookup_Mul0Layout_pcAdd(b: BoundLayout_Mul0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct DivInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
}
struct BoundLayout_DivInputLayout {
  lyt: DivInputLayout,
  buf: u32,
}
fn lookup_DivInputLayout_decoded(b: BoundLayout_DivInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_DivInputLayout_sourceRegs(b: BoundLayout_DivInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
alias ArgU16Layout16LayoutArray = array<ArgU16Layout, 16>;
struct BoundLayout_ArgU16Layout16LayoutArray {
  lyt: ArgU16Layout16LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout16LayoutArray(b: BoundLayout_ArgU16Layout16LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_Div0MulOutputLayout {
  argU16: ArgU16Layout16LayoutArray,
  argU8: ArgU8Layout13LayoutArray,
}
struct BoundLayout__Arguments_Div0MulOutputLayout {
  lyt: _Arguments_Div0MulOutputLayout,
  buf: u32,
}
fn lookup__Arguments_Div0MulOutputLayout_argU16(b: BoundLayout__Arguments_Div0MulOutputLayout) -> BoundLayout_ArgU16Layout16LayoutArray {
  return BoundLayout_ArgU16Layout16LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Div0MulOutputLayout_argU8(b: BoundLayout__Arguments_Div0MulOutputLayout) -> BoundLayout_ArgU8Layout13LayoutArray {
  return BoundLayout_ArgU8Layout13LayoutArray(b.lyt.argU8, b.buf);
}
struct DoDivLayout {
  quotLow: NondetRegLayout,
  quotHigh: NondetRegLayout,
  remLow: NondetU16RegLayout,
  remHigh: NondetU16RegLayout,
  mul: MultiplyAccumulateLayout,
  topBitType: NondetRegLayout,
  topNum: NondetRegLayout,
  _0: NondetU16RegLayout,
  denomAbs: NormalizeU32Layout,
  remNormal: NormalizeU32Layout,
  isZero: NondetRegLayout,
  signedOverflowCase: NondetRegLayout,
  lt: CmpLessThanUnsignedLayout,
}
struct BoundLayout_DoDivLayout {
  lyt: DoDivLayout,
  buf: u32,
}
fn lookup_DoDivLayout_quotLow(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.quotLow, b.buf);
}
fn lookup_DoDivLayout_quotHigh(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.quotHigh, b.buf);
}
fn lookup_DoDivLayout_remLow(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.remLow, b.buf);
}
fn lookup_DoDivLayout_remHigh(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.remHigh, b.buf);
}
fn lookup_DoDivLayout_mul(b: BoundLayout_DoDivLayout) -> BoundLayout_MultiplyAccumulateLayout {
  return BoundLayout_MultiplyAccumulateLayout(b.lyt.mul, b.buf);
}
fn lookup_DoDivLayout_topBitType(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.topBitType, b.buf);
}
fn lookup_DoDivLayout_topNum(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.topNum, b.buf);
}
fn lookup_DoDivLayout__0(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt._0, b.buf);
}
fn lookup_DoDivLayout_denomAbs(b: BoundLayout_DoDivLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.denomAbs, b.buf);
}
fn lookup_DoDivLayout_remNormal(b: BoundLayout_DoDivLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.remNormal, b.buf);
}
fn lookup_DoDivLayout_isZero(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isZero, b.buf);
}
fn lookup_DoDivLayout_signedOverflowCase(b: BoundLayout_DoDivLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.signedOverflowCase, b.buf);
}
fn lookup_DoDivLayout_lt(b: BoundLayout_DoDivLayout) -> BoundLayout_CmpLessThanUnsignedLayout {
  return BoundLayout_CmpLessThanUnsignedLayout(b.lyt.lt, b.buf);
}
struct OpSRLLayout {
  shiftMul: DynPo2Layout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRLLayout {
  lyt: OpSRLLayout,
  buf: u32,
}
fn lookup_OpSRLLayout_shiftMul(b: BoundLayout_OpSRLLayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRLLayout__0(b: BoundLayout_OpSRLLayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm0Layout {
  _super: OpSRLLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm0Layout {
  lyt: Div0MulOutputArm0Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm0Layout__super(b: BoundLayout_Div0MulOutputArm0Layout) -> BoundLayout_OpSRLLayout {
  return BoundLayout_OpSRLLayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm0Layout__extra0(b: BoundLayout_Div0MulOutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct TopBitLayout {
  _super: NondetRegLayout,
  rest: NondetU16RegLayout,
}
struct BoundLayout_TopBitLayout {
  lyt: TopBitLayout,
  buf: u32,
}
fn lookup_TopBitLayout__super(b: BoundLayout_TopBitLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_TopBitLayout_rest(b: BoundLayout_TopBitLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.rest, b.buf);
}
struct OpSRALayout {
  shiftMul: DynPo2Layout,
  flip: TopBitLayout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRALayout {
  lyt: OpSRALayout,
  buf: u32,
}
fn lookup_OpSRALayout_shiftMul(b: BoundLayout_OpSRALayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRALayout_flip(b: BoundLayout_OpSRALayout) -> BoundLayout_TopBitLayout {
  return BoundLayout_TopBitLayout(b.lyt.flip, b.buf);
}
fn lookup_OpSRALayout__0(b: BoundLayout_OpSRALayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct OpSRLILayout {
  shiftMul: DynPo2Layout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRLILayout {
  lyt: OpSRLILayout,
  buf: u32,
}
fn lookup_OpSRLILayout_shiftMul(b: BoundLayout_OpSRLILayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRLILayout__0(b: BoundLayout_OpSRLILayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm2Layout {
  _super: OpSRLILayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm2Layout {
  lyt: Div0MulOutputArm2Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm2Layout__super(b: BoundLayout_Div0MulOutputArm2Layout) -> BoundLayout_OpSRLILayout {
  return BoundLayout_OpSRLILayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm2Layout__extra0(b: BoundLayout_Div0MulOutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpSRAILayout {
  shiftMul: DynPo2Layout,
  flip: TopBitLayout,
  _0: DoDivLayout,
}
struct BoundLayout_OpSRAILayout {
  lyt: OpSRAILayout,
  buf: u32,
}
fn lookup_OpSRAILayout_shiftMul(b: BoundLayout_OpSRAILayout) -> BoundLayout_DynPo2Layout {
  return BoundLayout_DynPo2Layout(b.lyt.shiftMul, b.buf);
}
fn lookup_OpSRAILayout_flip(b: BoundLayout_OpSRAILayout) -> BoundLayout_TopBitLayout {
  return BoundLayout_TopBitLayout(b.lyt.flip, b.buf);
}
fn lookup_OpSRAILayout__0(b: BoundLayout_OpSRAILayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct OpDIVLayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpDIVLayout {
  lyt: OpDIVLayout,
  buf: u32,
}
fn lookup_OpDIVLayout__0(b: BoundLayout_OpDIVLayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm4Layout {
  _super: OpDIVLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm4Layout {
  lyt: Div0MulOutputArm4Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm4Layout__super(b: BoundLayout_Div0MulOutputArm4Layout) -> BoundLayout_OpDIVLayout {
  return BoundLayout_OpDIVLayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm4Layout__extra0(b: BoundLayout_Div0MulOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm4Layout__extra1(b: BoundLayout_Div0MulOutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct OpDIVULayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpDIVULayout {
  lyt: OpDIVULayout,
  buf: u32,
}
fn lookup_OpDIVULayout__0(b: BoundLayout_OpDIVULayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm5Layout {
  _super: OpDIVULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm5Layout {
  lyt: Div0MulOutputArm5Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm5Layout__super(b: BoundLayout_Div0MulOutputArm5Layout) -> BoundLayout_OpDIVULayout {
  return BoundLayout_OpDIVULayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm5Layout__extra0(b: BoundLayout_Div0MulOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm5Layout__extra1(b: BoundLayout_Div0MulOutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct OpREMLayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpREMLayout {
  lyt: OpREMLayout,
  buf: u32,
}
fn lookup_OpREMLayout__0(b: BoundLayout_OpREMLayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm6Layout {
  _super: OpREMLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm6Layout {
  lyt: Div0MulOutputArm6Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm6Layout__super(b: BoundLayout_Div0MulOutputArm6Layout) -> BoundLayout_OpREMLayout {
  return BoundLayout_OpREMLayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm6Layout__extra0(b: BoundLayout_Div0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm6Layout__extra1(b: BoundLayout_Div0MulOutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct OpREMULayout {
  _0: DoDivLayout,
}
struct BoundLayout_OpREMULayout {
  lyt: OpREMULayout,
  buf: u32,
}
fn lookup_OpREMULayout__0(b: BoundLayout_OpREMULayout) -> BoundLayout_DoDivLayout {
  return BoundLayout_DoDivLayout(b.lyt._0, b.buf);
}
struct Div0MulOutputArm7Layout {
  _super: OpREMULayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Div0MulOutputArm7Layout {
  lyt: Div0MulOutputArm7Layout,
  buf: u32,
}
fn lookup_Div0MulOutputArm7Layout__super(b: BoundLayout_Div0MulOutputArm7Layout) -> BoundLayout_OpREMULayout {
  return BoundLayout_OpREMULayout(b.lyt._super, b.buf);
}
fn lookup_Div0MulOutputArm7Layout__extra0(b: BoundLayout_Div0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Div0MulOutputArm7Layout__extra1(b: BoundLayout_Div0MulOutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct Div0MulOutputLayout {
  arm0: Div0MulOutputArm0Layout,
  arm1: OpSRALayout,
  arm2: Div0MulOutputArm2Layout,
  arm3: OpSRAILayout,
  arm4: Div0MulOutputArm4Layout,
  arm5: Div0MulOutputArm5Layout,
  arm6: Div0MulOutputArm6Layout,
  arm7: Div0MulOutputArm7Layout,
}
struct BoundLayout_Div0MulOutputLayout {
  lyt: Div0MulOutputLayout,
  buf: u32,
}
fn lookup_Div0MulOutputLayout_arm0(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm0Layout {
  return BoundLayout_Div0MulOutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Div0MulOutputLayout_arm1(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_OpSRALayout {
  return BoundLayout_OpSRALayout(b.lyt.arm1, b.buf);
}
fn lookup_Div0MulOutputLayout_arm2(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm2Layout {
  return BoundLayout_Div0MulOutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Div0MulOutputLayout_arm3(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_OpSRAILayout {
  return BoundLayout_OpSRAILayout(b.lyt.arm3, b.buf);
}
fn lookup_Div0MulOutputLayout_arm4(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm4Layout {
  return BoundLayout_Div0MulOutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Div0MulOutputLayout_arm5(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm5Layout {
  return BoundLayout_Div0MulOutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Div0MulOutputLayout_arm6(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm6Layout {
  return BoundLayout_Div0MulOutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Div0MulOutputLayout_arm7(b: BoundLayout_Div0MulOutputLayout) -> BoundLayout_Div0MulOutputArm7Layout {
  return BoundLayout_Div0MulOutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Div0Layout {
  _0: DoCycleTableLayout,
  input: DivInputLayout,
  _arguments_Div0MulOutput: _Arguments_Div0MulOutputLayout,
  mulOutput: Div0MulOutputLayout,
  _1: WriteRdLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Div0Layout {
  lyt: Div0Layout,
  buf: u32,
}
fn lookup_Div0Layout__0(b: BoundLayout_Div0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Div0Layout_input(b: BoundLayout_Div0Layout) -> BoundLayout_DivInputLayout {
  return BoundLayout_DivInputLayout(b.lyt.input, b.buf);
}
fn lookup_Div0Layout__arguments_Div0MulOutput(b: BoundLayout_Div0Layout) -> BoundLayout__Arguments_Div0MulOutputLayout {
  return BoundLayout__Arguments_Div0MulOutputLayout(b.lyt._arguments_Div0MulOutput, b.buf);
}
fn lookup_Div0Layout_mulOutput(b: BoundLayout_Div0Layout) -> BoundLayout_Div0MulOutputLayout {
  return BoundLayout_Div0MulOutputLayout(b.lyt.mulOutput, b.buf);
}
fn lookup_Div0Layout__1(b: BoundLayout_Div0Layout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._1, b.buf);
}
fn lookup_Div0Layout_pcAdd(b: BoundLayout_Div0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct AddrDecomposeBitsLayout {
  low0: NondetRegLayout,
  low1: NondetRegLayout,
  upperDiff: NondetU16RegLayout,
  _0: IsZeroLayout,
  med14: NondetU16RegLayout,
}
struct BoundLayout_AddrDecomposeBitsLayout {
  lyt: AddrDecomposeBitsLayout,
  buf: u32,
}
fn lookup_AddrDecomposeBitsLayout_low0(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low0, b.buf);
}
fn lookup_AddrDecomposeBitsLayout_low1(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low1, b.buf);
}
fn lookup_AddrDecomposeBitsLayout_upperDiff(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.upperDiff, b.buf);
}
fn lookup_AddrDecomposeBitsLayout__0(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
fn lookup_AddrDecomposeBitsLayout_med14(b: BoundLayout_AddrDecomposeBitsLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.med14, b.buf);
}
struct MemLoadInputLayout {
  decoded: DecodeInstLayout,
  rs1: ReadRegLayout,
  addrU32: NormalizeU32Layout,
  addr: AddrDecomposeBitsLayout,
  data: MemoryReadLayout,
}
struct BoundLayout_MemLoadInputLayout {
  lyt: MemLoadInputLayout,
  buf: u32,
}
fn lookup_MemLoadInputLayout_decoded(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MemLoadInputLayout_rs1(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_ReadRegLayout {
  return BoundLayout_ReadRegLayout(b.lyt.rs1, b.buf);
}
fn lookup_MemLoadInputLayout_addrU32(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.addrU32, b.buf);
}
fn lookup_MemLoadInputLayout_addr(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.addr, b.buf);
}
fn lookup_MemLoadInputLayout_data(b: BoundLayout_MemLoadInputLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.data, b.buf);
}
alias ArgU8Layout3LayoutArray = array<ArgU8Layout, 3>;
struct BoundLayout_ArgU8Layout3LayoutArray {
  lyt: ArgU8Layout3LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout3LayoutArray(b: BoundLayout_ArgU8Layout3LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
alias ArgU16Layout1LayoutArray = array<ArgU16Layout, 1>;
struct BoundLayout_ArgU16Layout1LayoutArray {
  lyt: ArgU16Layout1LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout1LayoutArray(b: BoundLayout_ArgU16Layout1LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_Mem0OutputLayout {
  argU8: ArgU8Layout3LayoutArray,
  argU16: ArgU16Layout1LayoutArray,
}
struct BoundLayout__Arguments_Mem0OutputLayout {
  lyt: _Arguments_Mem0OutputLayout,
  buf: u32,
}
fn lookup__Arguments_Mem0OutputLayout_argU8(b: BoundLayout__Arguments_Mem0OutputLayout) -> BoundLayout_ArgU8Layout3LayoutArray {
  return BoundLayout_ArgU8Layout3LayoutArray(b.lyt.argU8, b.buf);
}
fn lookup__Arguments_Mem0OutputLayout_argU16(b: BoundLayout__Arguments_Mem0OutputLayout) -> BoundLayout_ArgU16Layout1LayoutArray {
  return BoundLayout_ArgU16Layout1LayoutArray(b.lyt.argU16, b.buf);
}
struct SplitWordLayout {
  byte0: NondetU8RegLayout,
  byte1: NondetU8RegLayout,
}
struct BoundLayout_SplitWordLayout {
  lyt: SplitWordLayout,
  buf: u32,
}
fn lookup_SplitWordLayout_byte0(b: BoundLayout_SplitWordLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.byte0, b.buf);
}
fn lookup_SplitWordLayout_byte1(b: BoundLayout_SplitWordLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.byte1, b.buf);
}
struct OpLBLayout {
  bytes: SplitWordLayout,
  highBit: NondetRegLayout,
  low7x2: NondetU8RegLayout,
}
struct BoundLayout_OpLBLayout {
  lyt: OpLBLayout,
  buf: u32,
}
fn lookup_OpLBLayout_bytes(b: BoundLayout_OpLBLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.bytes, b.buf);
}
fn lookup_OpLBLayout_highBit(b: BoundLayout_OpLBLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.highBit, b.buf);
}
fn lookup_OpLBLayout_low7x2(b: BoundLayout_OpLBLayout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt.low7x2, b.buf);
}
struct Mem0OutputArm0Layout {
  _super: OpLBLayout,
  _extra0: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm0Layout {
  lyt: Mem0OutputArm0Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm0Layout__super(b: BoundLayout_Mem0OutputArm0Layout) -> BoundLayout_OpLBLayout {
  return BoundLayout_OpLBLayout(b.lyt._super, b.buf);
}
fn lookup_Mem0OutputArm0Layout__extra0(b: BoundLayout_Mem0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
struct OpLHLayout {
  highBit: NondetRegLayout,
  low15x2: NondetU16RegLayout,
}
struct BoundLayout_OpLHLayout {
  lyt: OpLHLayout,
  buf: u32,
}
fn lookup_OpLHLayout_highBit(b: BoundLayout_OpLHLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.highBit, b.buf);
}
fn lookup_OpLHLayout_low15x2(b: BoundLayout_OpLHLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low15x2, b.buf);
}
struct Mem0OutputArm1Layout {
  _super: OpLHLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
}
struct BoundLayout_Mem0OutputArm1Layout {
  lyt: Mem0OutputArm1Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm1Layout__super(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_OpLHLayout {
  return BoundLayout_OpLHLayout(b.lyt._super, b.buf);
}
fn lookup_Mem0OutputArm1Layout__extra0(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm1Layout__extra1(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm1Layout__extra2(b: BoundLayout_Mem0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
struct Mem0OutputArm2Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm2Layout {
  lyt: Mem0OutputArm2Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm2Layout__extra0(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm2Layout__extra1(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm2Layout__extra2(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm2Layout__extra3(b: BoundLayout_Mem0OutputArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct OpLBULayout {
  bytes: SplitWordLayout,
}
struct BoundLayout_OpLBULayout {
  lyt: OpLBULayout,
  buf: u32,
}
fn lookup_OpLBULayout_bytes(b: BoundLayout_OpLBULayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.bytes, b.buf);
}
struct Mem0OutputArm3Layout {
  _super: OpLBULayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm3Layout {
  lyt: Mem0OutputArm3Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm3Layout__super(b: BoundLayout_Mem0OutputArm3Layout) -> BoundLayout_OpLBULayout {
  return BoundLayout_OpLBULayout(b.lyt._super, b.buf);
}
fn lookup_Mem0OutputArm3Layout__extra0(b: BoundLayout_Mem0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm3Layout__extra1(b: BoundLayout_Mem0OutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct Mem0OutputArm4Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm4Layout {
  lyt: Mem0OutputArm4Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm4Layout__extra0(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm4Layout__extra1(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm4Layout__extra2(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm4Layout__extra3(b: BoundLayout_Mem0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputArm5Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm5Layout {
  lyt: Mem0OutputArm5Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm5Layout__extra0(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm5Layout__extra1(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm5Layout__extra2(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm5Layout__extra3(b: BoundLayout_Mem0OutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputArm6Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm6Layout {
  lyt: Mem0OutputArm6Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm6Layout__extra0(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm6Layout__extra1(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm6Layout__extra2(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm6Layout__extra3(b: BoundLayout_Mem0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputArm7Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU16Layout,
}
struct BoundLayout_Mem0OutputArm7Layout {
  lyt: Mem0OutputArm7Layout,
  buf: u32,
}
fn lookup_Mem0OutputArm7Layout__extra0(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem0OutputArm7Layout__extra1(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem0OutputArm7Layout__extra2(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem0OutputArm7Layout__extra3(b: BoundLayout_Mem0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
struct Mem0OutputLayout {
  arm0: Mem0OutputArm0Layout,
  arm1: Mem0OutputArm1Layout,
  arm2: Mem0OutputArm2Layout,
  arm3: Mem0OutputArm3Layout,
  arm4: Mem0OutputArm4Layout,
  arm5: Mem0OutputArm5Layout,
  arm6: Mem0OutputArm6Layout,
  arm7: Mem0OutputArm7Layout,
}
struct BoundLayout_Mem0OutputLayout {
  lyt: Mem0OutputLayout,
  buf: u32,
}
fn lookup_Mem0OutputLayout_arm0(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm0Layout {
  return BoundLayout_Mem0OutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Mem0OutputLayout_arm1(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm1Layout {
  return BoundLayout_Mem0OutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Mem0OutputLayout_arm2(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm2Layout {
  return BoundLayout_Mem0OutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Mem0OutputLayout_arm3(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm3Layout {
  return BoundLayout_Mem0OutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Mem0OutputLayout_arm4(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm4Layout {
  return BoundLayout_Mem0OutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Mem0OutputLayout_arm5(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm5Layout {
  return BoundLayout_Mem0OutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Mem0OutputLayout_arm6(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm6Layout {
  return BoundLayout_Mem0OutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Mem0OutputLayout_arm7(b: BoundLayout_Mem0OutputLayout) -> BoundLayout_Mem0OutputArm7Layout {
  return BoundLayout_Mem0OutputArm7Layout(b.lyt.arm7, b.buf);
}
struct Mem0Layout {
  _0: DoCycleTableLayout,
  input: MemLoadInputLayout,
  _arguments_Mem0Output: _Arguments_Mem0OutputLayout,
  output: Mem0OutputLayout,
  _1: WriteRdLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Mem0Layout {
  lyt: Mem0Layout,
  buf: u32,
}
fn lookup_Mem0Layout__0(b: BoundLayout_Mem0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Mem0Layout_input(b: BoundLayout_Mem0Layout) -> BoundLayout_MemLoadInputLayout {
  return BoundLayout_MemLoadInputLayout(b.lyt.input, b.buf);
}
fn lookup_Mem0Layout__arguments_Mem0Output(b: BoundLayout_Mem0Layout) -> BoundLayout__Arguments_Mem0OutputLayout {
  return BoundLayout__Arguments_Mem0OutputLayout(b.lyt._arguments_Mem0Output, b.buf);
}
fn lookup_Mem0Layout_output(b: BoundLayout_Mem0Layout) -> BoundLayout_Mem0OutputLayout {
  return BoundLayout_Mem0OutputLayout(b.lyt.output, b.buf);
}
fn lookup_Mem0Layout__1(b: BoundLayout_Mem0Layout) -> BoundLayout_WriteRdLayout {
  return BoundLayout_WriteRdLayout(b.lyt._1, b.buf);
}
fn lookup_Mem0Layout_pcAdd(b: BoundLayout_Mem0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct MemStoreInputLayout {
  decoded: DecodeInstLayout,
  sourceRegs: ReadSourceRegsLayout,
  addrU32: NormalizeU32Layout,
  addr: AddrDecomposeBitsLayout,
  data: MemoryReadLayout,
}
struct BoundLayout_MemStoreInputLayout {
  lyt: MemStoreInputLayout,
  buf: u32,
}
fn lookup_MemStoreInputLayout_decoded(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_DecodeInstLayout {
  return BoundLayout_DecodeInstLayout(b.lyt.decoded, b.buf);
}
fn lookup_MemStoreInputLayout_sourceRegs(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_ReadSourceRegsLayout {
  return BoundLayout_ReadSourceRegsLayout(b.lyt.sourceRegs, b.buf);
}
fn lookup_MemStoreInputLayout_addrU32(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.addrU32, b.buf);
}
fn lookup_MemStoreInputLayout_addr(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.addr, b.buf);
}
fn lookup_MemStoreInputLayout_data(b: BoundLayout_MemStoreInputLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.data, b.buf);
}
alias ArgU8Layout4LayoutArray = array<ArgU8Layout, 4>;
struct BoundLayout_ArgU8Layout4LayoutArray {
  lyt: ArgU8Layout4LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout4LayoutArray(b: BoundLayout_ArgU8Layout4LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_Mem1OutputLayout {
  argU8: ArgU8Layout4LayoutArray,
}
struct BoundLayout__Arguments_Mem1OutputLayout {
  lyt: _Arguments_Mem1OutputLayout,
  buf: u32,
}
fn lookup__Arguments_Mem1OutputLayout_argU8(b: BoundLayout__Arguments_Mem1OutputLayout) -> BoundLayout_ArgU8Layout4LayoutArray {
  return BoundLayout_ArgU8Layout4LayoutArray(b.lyt.argU8, b.buf);
}
struct OpSBLayout {
  origBytes: SplitWordLayout,
  newBytes: SplitWordLayout,
}
struct BoundLayout_OpSBLayout {
  lyt: OpSBLayout,
  buf: u32,
}
fn lookup_OpSBLayout_origBytes(b: BoundLayout_OpSBLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.origBytes, b.buf);
}
fn lookup_OpSBLayout_newBytes(b: BoundLayout_OpSBLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.newBytes, b.buf);
}
struct Mem1OutputArm1Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm1Layout {
  lyt: Mem1OutputArm1Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm1Layout__extra0(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm1Layout__extra1(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm1Layout__extra2(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm1Layout__extra3(b: BoundLayout_Mem1OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm2Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm2Layout {
  lyt: Mem1OutputArm2Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm2Layout__extra0(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm2Layout__extra1(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm2Layout__extra2(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm2Layout__extra3(b: BoundLayout_Mem1OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm3Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm3Layout {
  lyt: Mem1OutputArm3Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm3Layout__extra0(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm3Layout__extra1(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm3Layout__extra2(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm3Layout__extra3(b: BoundLayout_Mem1OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm4Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm4Layout {
  lyt: Mem1OutputArm4Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm4Layout__extra0(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm4Layout__extra1(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm4Layout__extra2(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm4Layout__extra3(b: BoundLayout_Mem1OutputArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm5Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm5Layout {
  lyt: Mem1OutputArm5Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm5Layout__extra0(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm5Layout__extra1(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm5Layout__extra2(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm5Layout__extra3(b: BoundLayout_Mem1OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm6Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm6Layout {
  lyt: Mem1OutputArm6Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm6Layout__extra0(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm6Layout__extra1(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm6Layout__extra2(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm6Layout__extra3(b: BoundLayout_Mem1OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputArm7Layout {
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_Mem1OutputArm7Layout {
  lyt: Mem1OutputArm7Layout,
  buf: u32,
}
fn lookup_Mem1OutputArm7Layout__extra0(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Mem1OutputArm7Layout__extra1(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_Mem1OutputArm7Layout__extra2(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_Mem1OutputArm7Layout__extra3(b: BoundLayout_Mem1OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct Mem1OutputLayout {
  arm0: OpSBLayout,
  arm1: Mem1OutputArm1Layout,
  arm2: Mem1OutputArm2Layout,
  arm3: Mem1OutputArm3Layout,
  arm4: Mem1OutputArm4Layout,
  arm5: Mem1OutputArm5Layout,
  arm6: Mem1OutputArm6Layout,
  arm7: Mem1OutputArm7Layout,
}
struct BoundLayout_Mem1OutputLayout {
  lyt: Mem1OutputLayout,
  buf: u32,
}
fn lookup_Mem1OutputLayout_arm0(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_OpSBLayout {
  return BoundLayout_OpSBLayout(b.lyt.arm0, b.buf);
}
fn lookup_Mem1OutputLayout_arm1(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm1Layout {
  return BoundLayout_Mem1OutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Mem1OutputLayout_arm2(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm2Layout {
  return BoundLayout_Mem1OutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Mem1OutputLayout_arm3(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm3Layout {
  return BoundLayout_Mem1OutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Mem1OutputLayout_arm4(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm4Layout {
  return BoundLayout_Mem1OutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Mem1OutputLayout_arm5(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm5Layout {
  return BoundLayout_Mem1OutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Mem1OutputLayout_arm6(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm6Layout {
  return BoundLayout_Mem1OutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Mem1OutputLayout_arm7(b: BoundLayout_Mem1OutputLayout) -> BoundLayout_Mem1OutputArm7Layout {
  return BoundLayout_Mem1OutputArm7Layout(b.lyt.arm7, b.buf);
}
struct MemStoreFinalizeLayout {
  _0: MemoryWriteLayout,
}
struct BoundLayout_MemStoreFinalizeLayout {
  lyt: MemStoreFinalizeLayout,
  buf: u32,
}
fn lookup_MemStoreFinalizeLayout__0(b: BoundLayout_MemStoreFinalizeLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct Mem1Layout {
  _0: DoCycleTableLayout,
  input: MemStoreInputLayout,
  _arguments_Mem1Output: _Arguments_Mem1OutputLayout,
  output: Mem1OutputLayout,
  _1: MemStoreFinalizeLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_Mem1Layout {
  lyt: Mem1Layout,
  buf: u32,
}
fn lookup_Mem1Layout__0(b: BoundLayout_Mem1Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Mem1Layout_input(b: BoundLayout_Mem1Layout) -> BoundLayout_MemStoreInputLayout {
  return BoundLayout_MemStoreInputLayout(b.lyt.input, b.buf);
}
fn lookup_Mem1Layout__arguments_Mem1Output(b: BoundLayout_Mem1Layout) -> BoundLayout__Arguments_Mem1OutputLayout {
  return BoundLayout__Arguments_Mem1OutputLayout(b.lyt._arguments_Mem1Output, b.buf);
}
fn lookup_Mem1Layout_output(b: BoundLayout_Mem1Layout) -> BoundLayout_Mem1OutputLayout {
  return BoundLayout_Mem1OutputLayout(b.lyt.output, b.buf);
}
fn lookup_Mem1Layout__1(b: BoundLayout_Mem1Layout) -> BoundLayout_MemStoreFinalizeLayout {
  return BoundLayout_MemStoreFinalizeLayout(b.lyt._1, b.buf);
}
fn lookup_Mem1Layout_pcAdd(b: BoundLayout_Mem1Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct MemoryPageInLayout {
  io: MemoryIOLayout,
}
struct BoundLayout_MemoryPageInLayout {
  lyt: MemoryPageInLayout,
  buf: u32,
}
fn lookup_MemoryPageInLayout_io(b: BoundLayout_MemoryPageInLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
alias MemoryPageInLayout8LayoutArray = array<MemoryPageInLayout, 8>;
struct BoundLayout_MemoryPageInLayout8LayoutArray {
  lyt: MemoryPageInLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryPageInLayout8LayoutArray(b: BoundLayout_MemoryPageInLayout8LayoutArray, i: u32) -> BoundLayout_MemoryPageInLayout {
  return BoundLayout_MemoryPageInLayout(b.lyt[i], b.buf);
}
struct ControlLoadRootAndNonceLayout {
  mem: MemoryPageInLayout8LayoutArray,
}
struct BoundLayout_ControlLoadRootAndNonceLayout {
  lyt: ControlLoadRootAndNonceLayout,
  buf: u32,
}
fn lookup_ControlLoadRootAndNonceLayout_mem(b: BoundLayout_ControlLoadRootAndNonceLayout) -> BoundLayout_MemoryPageInLayout8LayoutArray {
  return BoundLayout_MemoryPageInLayout8LayoutArray(b.lyt.mem, b.buf);
}
struct Control0_SuperArm0Layout {
  _super: ControlLoadRootAndNonceLayout,
  _extra0: CycleArgLayout,
  _extra1: CycleArgLayout,
  _extra2: CycleArgLayout,
  _extra3: CycleArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: CycleArgLayout,
  _extra7: CycleArgLayout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
  _extra38: ArgU8Layout,
  _extra39: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm0Layout {
  lyt: Control0_SuperArm0Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm0Layout__super(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ControlLoadRootAndNonceLayout {
  return BoundLayout_ControlLoadRootAndNonceLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra0(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra1(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra2(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra3(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra4(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra5(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra6(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra7(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra8(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra9(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra10(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra11(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra12(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra13(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra14(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra15(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra16(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra17(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra18(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra19(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra20(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra21(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra22(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra23(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra24(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra25(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra26(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra27(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra28(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra29(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra30(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra31(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra32(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra33(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra34(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra35(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra36(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra37(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra38(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm0Layout__extra39(b: BoundLayout_Control0_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
struct ControlResume_SuperArm0_SuperLayout {
  pc: MemoryReadLayout,
  mode: MemoryReadLayout,
}
struct BoundLayout_ControlResume_SuperArm0_SuperLayout {
  lyt: ControlResume_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm0_SuperLayout_pc(b: BoundLayout_ControlResume_SuperArm0_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.pc, b.buf);
}
fn lookup_ControlResume_SuperArm0_SuperLayout_mode(b: BoundLayout_ControlResume_SuperArm0_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.mode, b.buf);
}
struct ControlResume_SuperArm0Layout {
  _super: ControlResume_SuperArm0_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
}
struct BoundLayout_ControlResume_SuperArm0Layout {
  lyt: ControlResume_SuperArm0Layout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm0Layout__super(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_ControlResume_SuperArm0_SuperLayout {
  return BoundLayout_ControlResume_SuperArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra0(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra1(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra2(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra3(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra4(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra5(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra6(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra7(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra8(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra9(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra10(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra11(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra12(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra13(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra14(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra15(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra16(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_ControlResume_SuperArm0Layout__extra17(b: BoundLayout_ControlResume_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
struct ControlResume_SuperArm1_Super__0_SuperLayout {
  _0: MemoryWriteLayout,
}
struct BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout {
  lyt: ControlResume_SuperArm1_Super__0_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm1_Super__0_SuperLayout__0(b: BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
alias ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray = array<ControlResume_SuperArm1_Super__0_SuperLayout, 8>;
struct BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray {
  lyt: ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray(b: BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout {
  return BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout(b.lyt[i], b.buf);
}
struct ControlResume_SuperArm1_SuperLayout {
  _1: ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray,
}
struct BoundLayout_ControlResume_SuperArm1_SuperLayout {
  lyt: ControlResume_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperArm1_SuperLayout__1(b: BoundLayout_ControlResume_SuperArm1_SuperLayout) -> BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray {
  return BoundLayout_ControlResume_SuperArm1_Super__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
struct ControlResume_SuperLayout {
  arm0: ControlResume_SuperArm0Layout,
  arm1: ControlResume_SuperArm1_SuperLayout,
}
struct BoundLayout_ControlResume_SuperLayout {
  lyt: ControlResume_SuperLayout,
  buf: u32,
}
fn lookup_ControlResume_SuperLayout_arm0(b: BoundLayout_ControlResume_SuperLayout) -> BoundLayout_ControlResume_SuperArm0Layout {
  return BoundLayout_ControlResume_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ControlResume_SuperLayout_arm1(b: BoundLayout_ControlResume_SuperLayout) -> BoundLayout_ControlResume_SuperArm1_SuperLayout {
  return BoundLayout_ControlResume_SuperArm1_SuperLayout(b.lyt.arm1, b.buf);
}
alias MemoryArgLayout16LayoutArray = array<MemoryArgLayout, 16>;
struct BoundLayout_MemoryArgLayout16LayoutArray {
  lyt: MemoryArgLayout16LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout16LayoutArray(b: BoundLayout_MemoryArgLayout16LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout8LayoutArray = array<CycleArgLayout, 8>;
struct BoundLayout_CycleArgLayout8LayoutArray {
  lyt: CycleArgLayout8LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout8LayoutArray(b: BoundLayout_CycleArgLayout8LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_ControlResume_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
}
struct BoundLayout__Arguments_ControlResume_SuperLayout {
  lyt: _Arguments_ControlResume_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlResume_SuperLayout_memoryArg(b: BoundLayout__Arguments_ControlResume_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ControlResume_SuperLayout_cycleArg(b: BoundLayout__Arguments_ControlResume_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlResumeLayout {
  _super: ControlResume_SuperLayout,
  pcZero: IsZeroLayout,
  _arguments_ControlResume_Super: _Arguments_ControlResume_SuperLayout,
}
struct BoundLayout_ControlResumeLayout {
  lyt: ControlResumeLayout,
  buf: u32,
}
fn lookup_ControlResumeLayout__super(b: BoundLayout_ControlResumeLayout) -> BoundLayout_ControlResume_SuperLayout {
  return BoundLayout_ControlResume_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlResumeLayout_pcZero(b: BoundLayout_ControlResumeLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.pcZero, b.buf);
}
fn lookup_ControlResumeLayout__arguments_ControlResume_Super(b: BoundLayout_ControlResumeLayout) -> BoundLayout__Arguments_ControlResume_SuperLayout {
  return BoundLayout__Arguments_ControlResume_SuperLayout(b.lyt._arguments_ControlResume_Super, b.buf);
}
struct Control0_SuperArm1Layout {
  _super: ControlResumeLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm1Layout {
  lyt: Control0_SuperArm1Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm1Layout__super(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ControlResumeLayout {
  return BoundLayout_ControlResumeLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra0(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra1(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra2(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra3(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra4(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra5(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra6(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra7(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra8(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra9(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra10(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra11(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra12(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra13(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra14(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra15(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra16(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra17(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra18(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra19(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra20(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra21(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra22(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra23(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra24(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra25(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra26(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra27(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra28(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra29(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra30(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm1Layout__extra31(b: BoundLayout_Control0_SuperArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
struct ControlUserEcallOrFence_SuperArm0_SuperLayout {
  newPc: NormalizeU32Layout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout {
  lyt: ControlUserEcallOrFence_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm0_SuperLayout_newPc(b: BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.newPc, b.buf);
}
struct ControlUserEcallOrFence_SuperArm0Layout {
  _super: ControlUserEcallOrFence_SuperArm0_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm0Layout {
  lyt: ControlUserEcallOrFence_SuperArm0Layout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__super(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra0(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra1(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra2(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra3(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra4(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm0Layout__extra5(b: BoundLayout_ControlUserEcallOrFence_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
struct ControlUserEcallOrFence_SuperArm1_SuperLayout {
  newPcAddr: MemoryReadLayout,
  _0: MemoryWriteLayout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout {
  lyt: ControlUserEcallOrFence_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm1_SuperLayout_newPcAddr(b: BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.newPcAddr, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm1_SuperLayout__0(b: BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct ControlUserEcallOrFence_SuperArm1Layout {
  _super: ControlUserEcallOrFence_SuperArm1_SuperLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperArm1Layout {
  lyt: ControlUserEcallOrFence_SuperArm1Layout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperArm1Layout__super(b: BoundLayout_ControlUserEcallOrFence_SuperArm1Layout) -> BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm1_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm1Layout__extra0(b: BoundLayout_ControlUserEcallOrFence_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperArm1Layout__extra1(b: BoundLayout_ControlUserEcallOrFence_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
struct ControlUserEcallOrFence_SuperLayout {
  arm0: ControlUserEcallOrFence_SuperArm0Layout,
  arm1: ControlUserEcallOrFence_SuperArm1Layout,
}
struct BoundLayout_ControlUserEcallOrFence_SuperLayout {
  lyt: ControlUserEcallOrFence_SuperLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFence_SuperLayout_arm0(b: BoundLayout_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_ControlUserEcallOrFence_SuperArm0Layout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ControlUserEcallOrFence_SuperLayout_arm1(b: BoundLayout_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_ControlUserEcallOrFence_SuperArm1Layout {
  return BoundLayout_ControlUserEcallOrFence_SuperArm1Layout(b.lyt.arm1, b.buf);
}
alias ArgU16Layout2LayoutArray = array<ArgU16Layout, 2>;
struct BoundLayout_ArgU16Layout2LayoutArray {
  lyt: ArgU16Layout2LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout2LayoutArray(b: BoundLayout_ArgU16Layout2LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_ControlUserEcallOrFence_SuperLayout {
  argU16: ArgU16Layout2LayoutArray,
  memoryArg: MemoryArgLayout4LayoutArray,
  cycleArg: CycleArgLayout2LayoutArray,
}
struct BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout {
  lyt: _Arguments_ControlUserEcallOrFence_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlUserEcallOrFence_SuperLayout_argU16(b: BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_ArgU16Layout2LayoutArray {
  return BoundLayout_ArgU16Layout2LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_ControlUserEcallOrFence_SuperLayout_memoryArg(b: BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_MemoryArgLayout4LayoutArray {
  return BoundLayout_MemoryArgLayout4LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ControlUserEcallOrFence_SuperLayout_cycleArg(b: BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout) -> BoundLayout_CycleArgLayout2LayoutArray {
  return BoundLayout_CycleArgLayout2LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlUserEcallOrFenceLayout {
  _super: ControlUserEcallOrFence_SuperLayout,
  safeMode: NondetRegLayout,
  pcAddr: AddrDecomposeBitsLayout,
  loadInst: MemoryReadLayout,
  isFence: NondetRegLayout,
  _arguments_ControlUserEcallOrFence_Super: _Arguments_ControlUserEcallOrFence_SuperLayout,
}
struct BoundLayout_ControlUserEcallOrFenceLayout {
  lyt: ControlUserEcallOrFenceLayout,
  buf: u32,
}
fn lookup_ControlUserEcallOrFenceLayout__super(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_ControlUserEcallOrFence_SuperLayout {
  return BoundLayout_ControlUserEcallOrFence_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_safeMode(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.safeMode, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_pcAddr(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_loadInst(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout_isFence(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isFence, b.buf);
}
fn lookup_ControlUserEcallOrFenceLayout__arguments_ControlUserEcallOrFence_Super(b: BoundLayout_ControlUserEcallOrFenceLayout) -> BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout {
  return BoundLayout__Arguments_ControlUserEcallOrFence_SuperLayout(b.lyt._arguments_ControlUserEcallOrFence_Super, b.buf);
}
struct Control0_SuperArm2Layout {
  _super: ControlUserEcallOrFenceLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
  _extra38: ArgU8Layout,
  _extra39: ArgU8Layout,
  _extra40: ArgU8Layout,
  _extra41: ArgU8Layout,
  _extra42: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm2Layout {
  lyt: Control0_SuperArm2Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm2Layout__super(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ControlUserEcallOrFenceLayout {
  return BoundLayout_ControlUserEcallOrFenceLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra0(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra1(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra2(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra3(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra4(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra5(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra6(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra7(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra8(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra9(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra10(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra11(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra12(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra13(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra14(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra15(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra16(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra17(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra18(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra19(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra20(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra21(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra22(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra23(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra24(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra25(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra26(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra27(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra28(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra29(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra30(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra31(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra32(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra33(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra34(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra35(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra36(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra37(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra38(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra39(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra40(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra40, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra41(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra41, b.buf);
}
fn lookup_Control0_SuperArm2Layout__extra42(b: BoundLayout_Control0_SuperArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra42, b.buf);
}
struct ControlMRETLayout {
  safeMode: NondetRegLayout,
  pcAddr: AddrDecomposeBitsLayout,
  loadInst: MemoryReadLayout,
  pc: MemoryReadLayout,
  pcAdd: NormalizeU32Layout,
}
struct BoundLayout_ControlMRETLayout {
  lyt: ControlMRETLayout,
  buf: u32,
}
fn lookup_ControlMRETLayout_safeMode(b: BoundLayout_ControlMRETLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.safeMode, b.buf);
}
fn lookup_ControlMRETLayout_pcAddr(b: BoundLayout_ControlMRETLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_ControlMRETLayout_loadInst(b: BoundLayout_ControlMRETLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
fn lookup_ControlMRETLayout_pc(b: BoundLayout_ControlMRETLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.pc, b.buf);
}
fn lookup_ControlMRETLayout_pcAdd(b: BoundLayout_ControlMRETLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.pcAdd, b.buf);
}
struct Control0_SuperArm3Layout {
  _super: ControlMRETLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
  _extra38: ArgU8Layout,
  _extra39: ArgU8Layout,
  _extra40: ArgU8Layout,
  _extra41: ArgU8Layout,
  _extra42: ArgU8Layout,
  _extra43: ArgU8Layout,
  _extra44: ArgU8Layout,
  _extra45: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm3Layout {
  lyt: Control0_SuperArm3Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm3Layout__super(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ControlMRETLayout {
  return BoundLayout_ControlMRETLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra0(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra1(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra2(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra3(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra4(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra5(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra6(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra7(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra8(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra9(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra10(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra11(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra12(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra13(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra14(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra15(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra16(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra17(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra18(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra19(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra20(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra21(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra22(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra23(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra24(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra25(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra26(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra27(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra28(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra29(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra30(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra31(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra32(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra33(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra34(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra35(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra36(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra37(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra38(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra39(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra40(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra40, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra41(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra41, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra42(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra42, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra43(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra43, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra44(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra44, b.buf);
}
fn lookup_Control0_SuperArm3Layout__extra45(b: BoundLayout_Control0_SuperArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra45, b.buf);
}
alias MemoryReadLayout8LayoutArray = array<MemoryReadLayout, 8>;
struct BoundLayout_MemoryReadLayout8LayoutArray {
  lyt: MemoryReadLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryReadLayout8LayoutArray(b: BoundLayout_MemoryReadLayout8LayoutArray, i: u32) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt[i], b.buf);
}
struct ControlSuspend_SuperArm0_SuperLayout {
  _1: MemoryReadLayout8LayoutArray,
}
struct BoundLayout_ControlSuspend_SuperArm0_SuperLayout {
  lyt: ControlSuspend_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperArm0_SuperLayout__1(b: BoundLayout_ControlSuspend_SuperArm0_SuperLayout) -> BoundLayout_MemoryReadLayout8LayoutArray {
  return BoundLayout_MemoryReadLayout8LayoutArray(b.lyt._1, b.buf);
}
struct ControlSuspend_SuperArm1_SuperLayout {
  state: NondetRegLayout,
  _0: MemoryWriteLayout,
  _1: MemoryWriteLayout,
}
struct BoundLayout_ControlSuspend_SuperArm1_SuperLayout {
  lyt: ControlSuspend_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperArm1_SuperLayout_state(b: BoundLayout_ControlSuspend_SuperArm1_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.state, b.buf);
}
fn lookup_ControlSuspend_SuperArm1_SuperLayout__0(b: BoundLayout_ControlSuspend_SuperArm1_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
fn lookup_ControlSuspend_SuperArm1_SuperLayout__1(b: BoundLayout_ControlSuspend_SuperArm1_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._1, b.buf);
}
struct ControlSuspend_SuperArm1Layout {
  _super: ControlSuspend_SuperArm1_SuperLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
}
struct BoundLayout_ControlSuspend_SuperArm1Layout {
  lyt: ControlSuspend_SuperArm1Layout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperArm1Layout__super(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_ControlSuspend_SuperArm1_SuperLayout {
  return BoundLayout_ControlSuspend_SuperArm1_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra0(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra1(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra2(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra3(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra4(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra5(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra6(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra7(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra8(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra9(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra10(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra11(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra12(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra13(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra14(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra15(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra16(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_ControlSuspend_SuperArm1Layout__extra17(b: BoundLayout_ControlSuspend_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
struct ControlSuspend_SuperLayout {
  arm0: ControlSuspend_SuperArm0_SuperLayout,
  arm1: ControlSuspend_SuperArm1Layout,
}
struct BoundLayout_ControlSuspend_SuperLayout {
  lyt: ControlSuspend_SuperLayout,
  buf: u32,
}
fn lookup_ControlSuspend_SuperLayout_arm0(b: BoundLayout_ControlSuspend_SuperLayout) -> BoundLayout_ControlSuspend_SuperArm0_SuperLayout {
  return BoundLayout_ControlSuspend_SuperArm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_ControlSuspend_SuperLayout_arm1(b: BoundLayout_ControlSuspend_SuperLayout) -> BoundLayout_ControlSuspend_SuperArm1Layout {
  return BoundLayout_ControlSuspend_SuperArm1Layout(b.lyt.arm1, b.buf);
}
struct _Arguments_ControlSuspend_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
}
struct BoundLayout__Arguments_ControlSuspend_SuperLayout {
  lyt: _Arguments_ControlSuspend_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlSuspend_SuperLayout_memoryArg(b: BoundLayout__Arguments_ControlSuspend_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ControlSuspend_SuperLayout_cycleArg(b: BoundLayout__Arguments_ControlSuspend_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlSuspendLayout {
  _super: ControlSuspend_SuperLayout,
  pcZero: IsZeroLayout,
  _arguments_ControlSuspend_Super: _Arguments_ControlSuspend_SuperLayout,
}
struct BoundLayout_ControlSuspendLayout {
  lyt: ControlSuspendLayout,
  buf: u32,
}
fn lookup_ControlSuspendLayout__super(b: BoundLayout_ControlSuspendLayout) -> BoundLayout_ControlSuspend_SuperLayout {
  return BoundLayout_ControlSuspend_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlSuspendLayout_pcZero(b: BoundLayout_ControlSuspendLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.pcZero, b.buf);
}
fn lookup_ControlSuspendLayout__arguments_ControlSuspend_Super(b: BoundLayout_ControlSuspendLayout) -> BoundLayout__Arguments_ControlSuspend_SuperLayout {
  return BoundLayout__Arguments_ControlSuspend_SuperLayout(b.lyt._arguments_ControlSuspend_Super, b.buf);
}
struct Control0_SuperArm4Layout {
  _super: ControlSuspendLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm4Layout {
  lyt: Control0_SuperArm4Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm4Layout__super(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ControlSuspendLayout {
  return BoundLayout_ControlSuspendLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra0(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra1(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra2(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra3(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra4(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra5(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra6(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra7(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra8(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra9(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra10(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra11(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra12(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra13(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra14(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra15(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra16(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra17(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra18(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra19(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra20(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra21(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra22(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra23(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra24(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra25(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra26(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra27(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra28(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra29(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra30(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm4Layout__extra31(b: BoundLayout_Control0_SuperArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
struct MemoryPageOutLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryPageOutLayout {
  lyt: MemoryPageOutLayout,
  buf: u32,
}
fn lookup_MemoryPageOutLayout_io(b: BoundLayout_MemoryPageOutLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryPageOutLayout__0(b: BoundLayout_MemoryPageOutLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
alias MemoryPageOutLayout8LayoutArray = array<MemoryPageOutLayout, 8>;
struct BoundLayout_MemoryPageOutLayout8LayoutArray {
  lyt: MemoryPageOutLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryPageOutLayout8LayoutArray(b: BoundLayout_MemoryPageOutLayout8LayoutArray, i: u32) -> BoundLayout_MemoryPageOutLayout {
  return BoundLayout_MemoryPageOutLayout(b.lyt[i], b.buf);
}
struct ControlStoreRootLayout {
  _1: MemoryPageOutLayout8LayoutArray,
}
struct BoundLayout_ControlStoreRootLayout {
  lyt: ControlStoreRootLayout,
  buf: u32,
}
fn lookup_ControlStoreRootLayout__1(b: BoundLayout_ControlStoreRootLayout) -> BoundLayout_MemoryPageOutLayout8LayoutArray {
  return BoundLayout_MemoryPageOutLayout8LayoutArray(b.lyt._1, b.buf);
}
struct Control0_SuperArm5Layout {
  _super: ControlStoreRootLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm5Layout {
  lyt: Control0_SuperArm5Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm5Layout__super(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ControlStoreRootLayout {
  return BoundLayout_ControlStoreRootLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra0(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra1(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra2(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra3(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra4(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra5(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra6(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra7(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra8(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra9(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra10(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra11(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra12(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra13(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra14(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra15(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra16(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra17(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra18(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra19(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra20(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra21(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra22(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra23(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra24(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra25(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra26(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra27(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra28(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra29(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra30(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm5Layout__extra31(b: BoundLayout_Control0_SuperArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
struct ControlTable_SuperArm0_Super__0_SuperLayout {
  arg: ArgU16Layout,
}
struct BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout {
  lyt: ControlTable_SuperArm0_Super__0_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm0_Super__0_SuperLayout_arg(b: BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt.arg, b.buf);
}
alias ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray = array<ControlTable_SuperArm0_Super__0_SuperLayout, 16>;
struct BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray {
  lyt: ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray,
  buf: u32,
}
fn subscript_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray(b: BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray, i: u32) -> BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout {
  return BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout(b.lyt[i], b.buf);
}
struct ControlTable_SuperArm0_SuperLayout {
  _1: ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray,
  done: IsZeroLayout,
}
struct BoundLayout_ControlTable_SuperArm0_SuperLayout {
  lyt: ControlTable_SuperArm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm0_SuperLayout__1(b: BoundLayout_ControlTable_SuperArm0_SuperLayout) -> BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray {
  return BoundLayout_ControlTable_SuperArm0_Super__0_SuperLayout16LayoutArray(b.lyt._1, b.buf);
}
fn lookup_ControlTable_SuperArm0_SuperLayout_done(b: BoundLayout_ControlTable_SuperArm0_SuperLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.done, b.buf);
}
struct ControlTable_SuperArm0Layout {
  _super: ControlTable_SuperArm0_SuperLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
  _extra4: ArgU8Layout,
  _extra5: ArgU8Layout,
  _extra6: ArgU8Layout,
  _extra7: ArgU8Layout,
  _extra8: ArgU8Layout,
  _extra9: ArgU8Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
}
struct BoundLayout_ControlTable_SuperArm0Layout {
  lyt: ControlTable_SuperArm0Layout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm0Layout__super(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ControlTable_SuperArm0_SuperLayout {
  return BoundLayout_ControlTable_SuperArm0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra0(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra1(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra2(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra3(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra4(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra4, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra5(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra5, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra6(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra6, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra7(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra7, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra8(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra8, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra9(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra9, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra10(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra11(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra12(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra13(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra14(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_ControlTable_SuperArm0Layout__extra15(b: BoundLayout_ControlTable_SuperArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
struct ControlTable_SuperArm1_Super__0_SuperLayout {
  arg: ArgU8Layout,
}
struct BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout {
  lyt: ControlTable_SuperArm1_Super__0_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm1_Super__0_SuperLayout_arg(b: BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt.arg, b.buf);
}
alias ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray = array<ControlTable_SuperArm1_Super__0_SuperLayout, 16>;
struct BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray {
  lyt: ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray,
  buf: u32,
}
fn subscript_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray(b: BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray, i: u32) -> BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout {
  return BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout(b.lyt[i], b.buf);
}
struct ControlTable_SuperArm1_SuperLayout {
  _1: ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray,
  done: IsZeroLayout,
}
struct BoundLayout_ControlTable_SuperArm1_SuperLayout {
  lyt: ControlTable_SuperArm1_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm1_SuperLayout__1(b: BoundLayout_ControlTable_SuperArm1_SuperLayout) -> BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray {
  return BoundLayout_ControlTable_SuperArm1_Super__0_SuperLayout16LayoutArray(b.lyt._1, b.buf);
}
fn lookup_ControlTable_SuperArm1_SuperLayout_done(b: BoundLayout_ControlTable_SuperArm1_SuperLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.done, b.buf);
}
struct ControlTable_SuperArm1Layout {
  _super: ControlTable_SuperArm1_SuperLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
}
struct BoundLayout_ControlTable_SuperArm1Layout {
  lyt: ControlTable_SuperArm1Layout,
  buf: u32,
}
fn lookup_ControlTable_SuperArm1Layout__super(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ControlTable_SuperArm1_SuperLayout {
  return BoundLayout_ControlTable_SuperArm1_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra0(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra1(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra2(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra3(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra4(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra5(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra6(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra7(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra8(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra9(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra10(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra11(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra12(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra13(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra14(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_ControlTable_SuperArm1Layout__extra15(b: BoundLayout_ControlTable_SuperArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
struct ControlTable_SuperLayout {
  arm0: ControlTable_SuperArm0Layout,
  arm1: ControlTable_SuperArm1Layout,
}
struct BoundLayout_ControlTable_SuperLayout {
  lyt: ControlTable_SuperLayout,
  buf: u32,
}
fn lookup_ControlTable_SuperLayout_arm0(b: BoundLayout_ControlTable_SuperLayout) -> BoundLayout_ControlTable_SuperArm0Layout {
  return BoundLayout_ControlTable_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ControlTable_SuperLayout_arm1(b: BoundLayout_ControlTable_SuperLayout) -> BoundLayout_ControlTable_SuperArm1Layout {
  return BoundLayout_ControlTable_SuperArm1Layout(b.lyt.arm1, b.buf);
}
alias ArgU8Layout16LayoutArray = array<ArgU8Layout, 16>;
struct BoundLayout_ArgU8Layout16LayoutArray {
  lyt: ArgU8Layout16LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout16LayoutArray(b: BoundLayout_ArgU8Layout16LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_ControlTable_SuperLayout {
  argU16: ArgU16Layout16LayoutArray,
  argU8: ArgU8Layout16LayoutArray,
}
struct BoundLayout__Arguments_ControlTable_SuperLayout {
  lyt: _Arguments_ControlTable_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_ControlTable_SuperLayout_argU16(b: BoundLayout__Arguments_ControlTable_SuperLayout) -> BoundLayout_ArgU16Layout16LayoutArray {
  return BoundLayout_ArgU16Layout16LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_ControlTable_SuperLayout_argU8(b: BoundLayout__Arguments_ControlTable_SuperLayout) -> BoundLayout_ArgU8Layout16LayoutArray {
  return BoundLayout_ArgU8Layout16LayoutArray(b.lyt.argU8, b.buf);
}
struct ControlTableLayout {
  _super: ControlTable_SuperLayout,
  entry: NondetRegLayout,
  mode: NondetRegLayout,
  _arguments_ControlTable_Super: _Arguments_ControlTable_SuperLayout,
}
struct BoundLayout_ControlTableLayout {
  lyt: ControlTableLayout,
  buf: u32,
}
fn lookup_ControlTableLayout__super(b: BoundLayout_ControlTableLayout) -> BoundLayout_ControlTable_SuperLayout {
  return BoundLayout_ControlTable_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_ControlTableLayout_entry(b: BoundLayout_ControlTableLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.entry, b.buf);
}
fn lookup_ControlTableLayout_mode(b: BoundLayout_ControlTableLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.mode, b.buf);
}
fn lookup_ControlTableLayout__arguments_ControlTable_Super(b: BoundLayout_ControlTableLayout) -> BoundLayout__Arguments_ControlTable_SuperLayout {
  return BoundLayout__Arguments_ControlTable_SuperLayout(b.lyt._arguments_ControlTable_Super, b.buf);
}
struct Control0_SuperArm6Layout {
  _super: ControlTableLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
}
struct BoundLayout_Control0_SuperArm6Layout {
  lyt: Control0_SuperArm6Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm6Layout__super(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_ControlTableLayout {
  return BoundLayout_ControlTableLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra0(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra1(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra2(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra3(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra4(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra5(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra6(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra7(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra8(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra9(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra10(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra11(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra12(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra13(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra14(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra15(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra16(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra17(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra18(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra19(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra20(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra21(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra22(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm6Layout__extra23(b: BoundLayout_Control0_SuperArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
alias CycleArgLayout1LayoutArray = array<CycleArgLayout, 1>;
struct BoundLayout_CycleArgLayout1LayoutArray {
  lyt: CycleArgLayout1LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout1LayoutArray(b: BoundLayout_CycleArgLayout1LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_ControlDone__0Layout {
  cycleArg: CycleArgLayout1LayoutArray,
}
struct BoundLayout__Arguments_ControlDone__0Layout {
  lyt: _Arguments_ControlDone__0Layout,
  buf: u32,
}
fn lookup__Arguments_ControlDone__0Layout_cycleArg(b: BoundLayout__Arguments_ControlDone__0Layout) -> BoundLayout_CycleArgLayout1LayoutArray {
  return BoundLayout_CycleArgLayout1LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ControlDone__0Arm0_SuperLayout {
  _0: IsCycleLayout,
}
struct BoundLayout_ControlDone__0Arm0_SuperLayout {
  lyt: ControlDone__0Arm0_SuperLayout,
  buf: u32,
}
fn lookup_ControlDone__0Arm0_SuperLayout__0(b: BoundLayout_ControlDone__0Arm0_SuperLayout) -> BoundLayout_IsCycleLayout {
  return BoundLayout_IsCycleLayout(b.lyt._0, b.buf);
}
struct ControlDone__0Arm1Layout {
  _extra0: CycleArgLayout,
}
struct BoundLayout_ControlDone__0Arm1Layout {
  lyt: ControlDone__0Arm1Layout,
  buf: u32,
}
fn lookup_ControlDone__0Arm1Layout__extra0(b: BoundLayout_ControlDone__0Arm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra0, b.buf);
}
struct ControlDone__0Layout {
  arm0: ControlDone__0Arm0_SuperLayout,
  arm1: ControlDone__0Arm1Layout,
}
struct BoundLayout_ControlDone__0Layout {
  lyt: ControlDone__0Layout,
  buf: u32,
}
fn lookup_ControlDone__0Layout_arm0(b: BoundLayout_ControlDone__0Layout) -> BoundLayout_ControlDone__0Arm0_SuperLayout {
  return BoundLayout_ControlDone__0Arm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_ControlDone__0Layout_arm1(b: BoundLayout_ControlDone__0Layout) -> BoundLayout_ControlDone__0Arm1Layout {
  return BoundLayout_ControlDone__0Arm1Layout(b.lyt.arm1, b.buf);
}
struct ControlDoneLayout {
  _arguments_ControlDone__0: _Arguments_ControlDone__0Layout,
  _2: ControlDone__0Layout,
}
struct BoundLayout_ControlDoneLayout {
  lyt: ControlDoneLayout,
  buf: u32,
}
fn lookup_ControlDoneLayout__arguments_ControlDone__0(b: BoundLayout_ControlDoneLayout) -> BoundLayout__Arguments_ControlDone__0Layout {
  return BoundLayout__Arguments_ControlDone__0Layout(b.lyt._arguments_ControlDone__0, b.buf);
}
fn lookup_ControlDoneLayout__2(b: BoundLayout_ControlDoneLayout) -> BoundLayout_ControlDone__0Layout {
  return BoundLayout_ControlDone__0Layout(b.lyt._2, b.buf);
}
struct Control0_SuperArm7Layout {
  _super: ControlDoneLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU8Layout,
  _extra40: ArgU8Layout,
  _extra41: ArgU8Layout,
  _extra42: ArgU8Layout,
  _extra43: ArgU8Layout,
  _extra44: ArgU8Layout,
  _extra45: ArgU8Layout,
  _extra46: ArgU8Layout,
  _extra47: ArgU8Layout,
  _extra48: ArgU8Layout,
  _extra49: ArgU8Layout,
  _extra50: ArgU8Layout,
  _extra51: ArgU8Layout,
  _extra52: ArgU8Layout,
  _extra53: ArgU8Layout,
  _extra54: ArgU8Layout,
}
struct BoundLayout_Control0_SuperArm7Layout {
  lyt: Control0_SuperArm7Layout,
  buf: u32,
}
fn lookup_Control0_SuperArm7Layout__super(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ControlDoneLayout {
  return BoundLayout_ControlDoneLayout(b.lyt._super, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra0(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra1(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra2(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra3(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra4(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra5(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra6(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra7(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra8(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra9(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra10(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra11(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra12(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra13(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra14(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra15(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra16(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra17(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra18(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra19(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra20(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra21(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra22(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra23(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra24(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra25(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra26(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra27(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra28(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra29(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra30(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra31(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra32(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra33(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra34(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra35(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra36(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra37(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra38(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra39(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra39, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra40(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra40, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra41(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra41, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra42(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra42, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra43(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra43, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra44(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra44, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra45(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra45, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra46(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra46, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra47(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra47, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra48(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra48, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra49(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra49, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra50(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra50, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra51(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra51, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra52(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra52, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra53(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra53, b.buf);
}
fn lookup_Control0_SuperArm7Layout__extra54(b: BoundLayout_Control0_SuperArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra54, b.buf);
}
struct Control0_SuperLayout {
  arm0: Control0_SuperArm0Layout,
  arm1: Control0_SuperArm1Layout,
  arm2: Control0_SuperArm2Layout,
  arm3: Control0_SuperArm3Layout,
  arm4: Control0_SuperArm4Layout,
  arm5: Control0_SuperArm5Layout,
  arm6: Control0_SuperArm6Layout,
  arm7: Control0_SuperArm7Layout,
}
struct BoundLayout_Control0_SuperLayout {
  lyt: Control0_SuperLayout,
  buf: u32,
}
fn lookup_Control0_SuperLayout_arm0(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm0Layout {
  return BoundLayout_Control0_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Control0_SuperLayout_arm1(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm1Layout {
  return BoundLayout_Control0_SuperArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Control0_SuperLayout_arm2(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm2Layout {
  return BoundLayout_Control0_SuperArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Control0_SuperLayout_arm3(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm3Layout {
  return BoundLayout_Control0_SuperArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Control0_SuperLayout_arm4(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm4Layout {
  return BoundLayout_Control0_SuperArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Control0_SuperLayout_arm5(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm5Layout {
  return BoundLayout_Control0_SuperArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Control0_SuperLayout_arm6(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm6Layout {
  return BoundLayout_Control0_SuperArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Control0_SuperLayout_arm7(b: BoundLayout_Control0_SuperLayout) -> BoundLayout_Control0_SuperArm7Layout {
  return BoundLayout_Control0_SuperArm7Layout(b.lyt.arm7, b.buf);
}
struct _Arguments_Control0_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
  argU16: ArgU16Layout16LayoutArray,
  argU8: ArgU8Layout16LayoutArray,
}
struct BoundLayout__Arguments_Control0_SuperLayout {
  lyt: _Arguments_Control0_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_Control0_SuperLayout_memoryArg(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_Control0_SuperLayout_cycleArg(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_Control0_SuperLayout_argU16(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_ArgU16Layout16LayoutArray {
  return BoundLayout_ArgU16Layout16LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Control0_SuperLayout_argU8(b: BoundLayout__Arguments_Control0_SuperLayout) -> BoundLayout_ArgU8Layout16LayoutArray {
  return BoundLayout_ArgU8Layout16LayoutArray(b.lyt.argU8, b.buf);
}
struct Control0Layout {
  _super: Control0_SuperLayout,
  _0: DoCycleTableLayout,
  _arguments_Control0_Super: _Arguments_Control0_SuperLayout,
}
struct BoundLayout_Control0Layout {
  lyt: Control0Layout,
  buf: u32,
}
fn lookup_Control0Layout__super(b: BoundLayout_Control0Layout) -> BoundLayout_Control0_SuperLayout {
  return BoundLayout_Control0_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_Control0Layout__0(b: BoundLayout_Control0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Control0Layout__arguments_Control0_Super(b: BoundLayout_Control0Layout) -> BoundLayout__Arguments_Control0_SuperLayout {
  return BoundLayout__Arguments_Control0_SuperLayout(b.lyt._arguments_Control0_Super, b.buf);
}
alias MemoryArgLayout8LayoutArray = array<MemoryArgLayout, 8>;
struct BoundLayout_MemoryArgLayout8LayoutArray {
  lyt: MemoryArgLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout8LayoutArray(b: BoundLayout_MemoryArgLayout8LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout4LayoutArray = array<CycleArgLayout, 4>;
struct BoundLayout_CycleArgLayout4LayoutArray {
  lyt: CycleArgLayout4LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout4LayoutArray(b: BoundLayout_CycleArgLayout4LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
alias ArgU16Layout4LayoutArray = array<ArgU16Layout, 4>;
struct BoundLayout_ArgU16Layout4LayoutArray {
  lyt: ArgU16Layout4LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout4LayoutArray(b: BoundLayout_ArgU16Layout4LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
struct _Arguments_ECall0OutputLayout {
  memoryArg: MemoryArgLayout8LayoutArray,
  cycleArg: CycleArgLayout4LayoutArray,
  argU16: ArgU16Layout4LayoutArray,
  argU8: ArgU8Layout4LayoutArray,
}
struct BoundLayout__Arguments_ECall0OutputLayout {
  lyt: _Arguments_ECall0OutputLayout,
  buf: u32,
}
fn lookup__Arguments_ECall0OutputLayout_memoryArg(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_MemoryArgLayout8LayoutArray {
  return BoundLayout_MemoryArgLayout8LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_ECall0OutputLayout_cycleArg(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_CycleArgLayout4LayoutArray {
  return BoundLayout_CycleArgLayout4LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_ECall0OutputLayout_argU16(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_ArgU16Layout4LayoutArray {
  return BoundLayout_ArgU16Layout4LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_ECall0OutputLayout_argU8(b: BoundLayout__Arguments_ECall0OutputLayout) -> BoundLayout_ArgU8Layout4LayoutArray {
  return BoundLayout_ArgU8Layout4LayoutArray(b.lyt.argU8, b.buf);
}
alias NondetRegLayout6LayoutArray = array<NondetRegLayout, 6>;
struct BoundLayout_NondetRegLayout6LayoutArray {
  lyt: NondetRegLayout6LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout6LayoutArray(b: BoundLayout_NondetRegLayout6LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_6_Layout {
  _super: NondetRegLayout6LayoutArray,
}
struct BoundLayout_OneHot_6_Layout {
  lyt: OneHot_6_Layout,
  buf: u32,
}
fn lookup_OneHot_6_Layout__super(b: BoundLayout_OneHot_6_Layout) -> BoundLayout_NondetRegLayout6LayoutArray {
  return BoundLayout_NondetRegLayout6LayoutArray(b.lyt._super, b.buf);
}
struct MachineECallLayout {
  loadInst: MemoryReadLayout,
  dispatchIdx: MemoryReadLayout,
  dispatch: OneHot_6_Layout,
}
struct BoundLayout_MachineECallLayout {
  lyt: MachineECallLayout,
  buf: u32,
}
fn lookup_MachineECallLayout_loadInst(b: BoundLayout_MachineECallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst, b.buf);
}
fn lookup_MachineECallLayout_dispatchIdx(b: BoundLayout_MachineECallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.dispatchIdx, b.buf);
}
fn lookup_MachineECallLayout_dispatch(b: BoundLayout_MachineECallLayout) -> BoundLayout_OneHot_6_Layout {
  return BoundLayout_OneHot_6_Layout(b.lyt.dispatch, b.buf);
}
struct ECall0OutputArm0Layout {
  _super: MachineECallLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm0Layout {
  lyt: ECall0OutputArm0Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm0Layout__super(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MachineECallLayout {
  return BoundLayout_MachineECallLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra0(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra1(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra2(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra3(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra4(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra5(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra6(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra7(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra8(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra9(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra10(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra11(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra12(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm0Layout__extra13(b: BoundLayout_ECall0OutputArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
struct ECallTerminateLayout {
  a0: MemoryReadLayout,
  a1: MemoryReadLayout,
}
struct BoundLayout_ECallTerminateLayout {
  lyt: ECallTerminateLayout,
  buf: u32,
}
fn lookup_ECallTerminateLayout_a0(b: BoundLayout_ECallTerminateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.a0, b.buf);
}
fn lookup_ECallTerminateLayout_a1(b: BoundLayout_ECallTerminateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.a1, b.buf);
}
struct ECall0OutputArm1Layout {
  _super: ECallTerminateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU8Layout,
  _extra11: ArgU8Layout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm1Layout {
  lyt: ECall0OutputArm1Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm1Layout__super(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ECallTerminateLayout {
  return BoundLayout_ECallTerminateLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra0(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra1(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra2(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra3(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra4(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra5(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra6(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra7(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra8(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra9(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra10(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra11(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra12(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm1Layout__extra13(b: BoundLayout_ECall0OutputArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
alias NondetRegLayout4LayoutArray = array<NondetRegLayout, 4>;
struct BoundLayout_NondetRegLayout4LayoutArray {
  lyt: NondetRegLayout4LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout4LayoutArray(b: BoundLayout_NondetRegLayout4LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_4_Layout {
  _super: NondetRegLayout4LayoutArray,
}
struct BoundLayout_OneHot_4_Layout {
  lyt: OneHot_4_Layout,
  buf: u32,
}
fn lookup_OneHot_4_Layout__super(b: BoundLayout_OneHot_4_Layout) -> BoundLayout_NondetRegLayout4LayoutArray {
  return BoundLayout_NondetRegLayout4LayoutArray(b.lyt._super, b.buf);
}
struct DecomposeLow2Layout {
  high: NondetU16RegLayout,
  low2: NondetRegLayout,
  low2Hot: OneHot_4_Layout,
  highZero: IsZeroLayout,
  isZero: NondetRegLayout,
}
struct BoundLayout_DecomposeLow2Layout {
  lyt: DecomposeLow2Layout,
  buf: u32,
}
fn lookup_DecomposeLow2Layout_high(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.high, b.buf);
}
fn lookup_DecomposeLow2Layout_low2(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low2, b.buf);
}
fn lookup_DecomposeLow2Layout_low2Hot(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_OneHot_4_Layout {
  return BoundLayout_OneHot_4_Layout(b.lyt.low2Hot, b.buf);
}
fn lookup_DecomposeLow2Layout_highZero(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.highZero, b.buf);
}
fn lookup_DecomposeLow2Layout_isZero(b: BoundLayout_DecomposeLow2Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isZero, b.buf);
}
struct ECallHostReadSetupLayout {
  fd: MemoryReadLayout,
  ptr_: MemoryReadLayout,
  len: MemoryReadLayout,
  newLen: NondetU16RegLayout,
  diff: NondetU16RegLayout,
  _0: MemoryWriteLayout,
  ptrDecomp: DecomposeLow2Layout,
  lenDecomp: DecomposeLow2Layout,
  len123: NondetRegLayout,
  uneven: NondetRegLayout,
}
struct BoundLayout_ECallHostReadSetupLayout {
  lyt: ECallHostReadSetupLayout,
  buf: u32,
}
fn lookup_ECallHostReadSetupLayout_fd(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.fd, b.buf);
}
fn lookup_ECallHostReadSetupLayout_ptr_(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.ptr_, b.buf);
}
fn lookup_ECallHostReadSetupLayout_len(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.len, b.buf);
}
fn lookup_ECallHostReadSetupLayout_newLen(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.newLen, b.buf);
}
fn lookup_ECallHostReadSetupLayout_diff(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.diff, b.buf);
}
fn lookup_ECallHostReadSetupLayout__0(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
fn lookup_ECallHostReadSetupLayout_ptrDecomp(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.ptrDecomp, b.buf);
}
fn lookup_ECallHostReadSetupLayout_lenDecomp(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.lenDecomp, b.buf);
}
fn lookup_ECallHostReadSetupLayout_len123(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.len123, b.buf);
}
fn lookup_ECallHostReadSetupLayout_uneven(b: BoundLayout_ECallHostReadSetupLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.uneven, b.buf);
}
struct ECall0OutputArm2Layout {
  _super: ECallHostReadSetupLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm2Layout {
  lyt: ECall0OutputArm2Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm2Layout__super(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ECallHostReadSetupLayout {
  return BoundLayout_ECallHostReadSetupLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra0(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra1(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra2(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm2Layout__extra3(b: BoundLayout_ECall0OutputArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
struct ECallHostWriteLayout {
  fd: MemoryReadLayout,
  ptr_: MemoryReadLayout,
  len: MemoryReadLayout,
  newLen: NondetU16RegLayout,
  diff: NondetU16RegLayout,
  _0: MemoryWriteLayout,
}
struct BoundLayout_ECallHostWriteLayout {
  lyt: ECallHostWriteLayout,
  buf: u32,
}
fn lookup_ECallHostWriteLayout_fd(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.fd, b.buf);
}
fn lookup_ECallHostWriteLayout_ptr_(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.ptr_, b.buf);
}
fn lookup_ECallHostWriteLayout_len(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.len, b.buf);
}
fn lookup_ECallHostWriteLayout_newLen(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.newLen, b.buf);
}
fn lookup_ECallHostWriteLayout_diff(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.diff, b.buf);
}
fn lookup_ECallHostWriteLayout__0(b: BoundLayout_ECallHostWriteLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
struct ECall0OutputArm3Layout {
  _super: ECallHostWriteLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
  _extra4: ArgU8Layout,
  _extra5: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm3Layout {
  lyt: ECall0OutputArm3Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm3Layout__super(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ECallHostWriteLayout {
  return BoundLayout_ECallHostWriteLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra0(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra1(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra2(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra3(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra4(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm3Layout__extra5(b: BoundLayout_ECall0OutputArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra5, b.buf);
}
struct MemoryWriteUnconstrainedLayout {
  io: MemoryIOLayout,
  _0: IsForwardLayout,
}
struct BoundLayout_MemoryWriteUnconstrainedLayout {
  lyt: MemoryWriteUnconstrainedLayout,
  buf: u32,
}
fn lookup_MemoryWriteUnconstrainedLayout_io(b: BoundLayout_MemoryWriteUnconstrainedLayout) -> BoundLayout_MemoryIOLayout {
  return BoundLayout_MemoryIOLayout(b.lyt.io, b.buf);
}
fn lookup_MemoryWriteUnconstrainedLayout__0(b: BoundLayout_MemoryWriteUnconstrainedLayout) -> BoundLayout_IsForwardLayout {
  return BoundLayout_IsForwardLayout(b.lyt._0, b.buf);
}
struct ECallHostReadBytesLayout {
  lenDecomp: DecomposeLow2Layout,
  len123: NondetRegLayout,
  nextPtrEven: IsZeroLayout,
  uneven: NondetRegLayout,
  lenZero: IsZeroLayout,
  low0: NondetRegLayout,
  low1: NondetRegLayout,
  origWord: MemoryReadLayout,
  _0: MemoryWriteUnconstrainedLayout,
  oldBytes: SplitWordLayout,
  newBytes_0: SplitWordLayout,
}
struct BoundLayout_ECallHostReadBytesLayout {
  lyt: ECallHostReadBytesLayout,
  buf: u32,
}
fn lookup_ECallHostReadBytesLayout_lenDecomp(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.lenDecomp, b.buf);
}
fn lookup_ECallHostReadBytesLayout_len123(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.len123, b.buf);
}
fn lookup_ECallHostReadBytesLayout_nextPtrEven(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.nextPtrEven, b.buf);
}
fn lookup_ECallHostReadBytesLayout_uneven(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.uneven, b.buf);
}
fn lookup_ECallHostReadBytesLayout_lenZero(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lenZero, b.buf);
}
fn lookup_ECallHostReadBytesLayout_low0(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low0, b.buf);
}
fn lookup_ECallHostReadBytesLayout_low1(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low1, b.buf);
}
fn lookup_ECallHostReadBytesLayout_origWord(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.origWord, b.buf);
}
fn lookup_ECallHostReadBytesLayout__0(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_MemoryWriteUnconstrainedLayout {
  return BoundLayout_MemoryWriteUnconstrainedLayout(b.lyt._0, b.buf);
}
fn lookup_ECallHostReadBytesLayout_oldBytes(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.oldBytes, b.buf);
}
fn lookup_ECallHostReadBytesLayout_newBytes_0(b: BoundLayout_ECallHostReadBytesLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.newBytes_0, b.buf);
}
struct ECall0OutputArm4Layout {
  _super: ECallHostReadBytesLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: CycleArgLayout,
  _extra5: CycleArgLayout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
}
struct BoundLayout_ECall0OutputArm4Layout {
  lyt: ECall0OutputArm4Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm4Layout__super(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ECallHostReadBytesLayout {
  return BoundLayout_ECallHostReadBytesLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra0(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra1(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra2(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra3(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra4(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra5(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra6(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra7(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm4Layout__extra8(b: BoundLayout_ECall0OutputArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
struct ECallHostReadWords__0_SuperLayout {
  addr: NondetRegLayout,
  _0: MemoryWriteUnconstrainedLayout,
}
struct BoundLayout_ECallHostReadWords__0_SuperLayout {
  lyt: ECallHostReadWords__0_SuperLayout,
  buf: u32,
}
fn lookup_ECallHostReadWords__0_SuperLayout_addr(b: BoundLayout_ECallHostReadWords__0_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.addr, b.buf);
}
fn lookup_ECallHostReadWords__0_SuperLayout__0(b: BoundLayout_ECallHostReadWords__0_SuperLayout) -> BoundLayout_MemoryWriteUnconstrainedLayout {
  return BoundLayout_MemoryWriteUnconstrainedLayout(b.lyt._0, b.buf);
}
alias ECallHostReadWords__0_SuperLayout4LayoutArray = array<ECallHostReadWords__0_SuperLayout, 4>;
struct BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray {
  lyt: ECallHostReadWords__0_SuperLayout4LayoutArray,
  buf: u32,
}
fn subscript_ECallHostReadWords__0_SuperLayout4LayoutArray(b: BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray, i: u32) -> BoundLayout_ECallHostReadWords__0_SuperLayout {
  return BoundLayout_ECallHostReadWords__0_SuperLayout(b.lyt[i], b.buf);
}
struct ECallHostReadWordsLayout {
  lenDecomp: DecomposeLow2Layout,
  wordsDecomp: DecomposeLow2Layout,
  _1: ECallHostReadWords__0_SuperLayout4LayoutArray,
  newLenHighZero: IsZeroLayout,
  lenZero: NondetRegLayout,
}
struct BoundLayout_ECallHostReadWordsLayout {
  lyt: ECallHostReadWordsLayout,
  buf: u32,
}
fn lookup_ECallHostReadWordsLayout_lenDecomp(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.lenDecomp, b.buf);
}
fn lookup_ECallHostReadWordsLayout_wordsDecomp(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_DecomposeLow2Layout {
  return BoundLayout_DecomposeLow2Layout(b.lyt.wordsDecomp, b.buf);
}
fn lookup_ECallHostReadWordsLayout__1(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray {
  return BoundLayout_ECallHostReadWords__0_SuperLayout4LayoutArray(b.lyt._1, b.buf);
}
fn lookup_ECallHostReadWordsLayout_newLenHighZero(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.newLenHighZero, b.buf);
}
fn lookup_ECallHostReadWordsLayout_lenZero(b: BoundLayout_ECallHostReadWordsLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lenZero, b.buf);
}
struct ECall0OutputArm5Layout {
  _super: ECallHostReadWordsLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU8Layout,
  _extra3: ArgU8Layout,
  _extra4: ArgU8Layout,
  _extra5: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm5Layout {
  lyt: ECall0OutputArm5Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm5Layout__super(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ECallHostReadWordsLayout {
  return BoundLayout_ECallHostReadWordsLayout(b.lyt._super, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra0(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra1(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra2(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra3(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra4(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm5Layout__extra5(b: BoundLayout_ECall0OutputArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra5, b.buf);
}
struct ECall0OutputArm6Layout {
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm6Layout {
  lyt: ECall0OutputArm6Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm6Layout__extra0(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra1(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra2(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra3(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra4(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra5(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra6(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra7(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra8(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra9(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra10(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra11(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra12(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra13(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra14(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra15(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra16(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra17(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra18(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_ECall0OutputArm6Layout__extra19(b: BoundLayout_ECall0OutputArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
struct ECall0OutputArm7Layout {
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
}
struct BoundLayout_ECall0OutputArm7Layout {
  lyt: ECall0OutputArm7Layout,
  buf: u32,
}
fn lookup_ECall0OutputArm7Layout__extra0(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra1(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra2(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra3(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra4(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra5(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra6(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra7(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra8(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra9(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra10(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra11(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra12(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra13(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra14(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra15(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra16(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra17(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra18(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_ECall0OutputArm7Layout__extra19(b: BoundLayout_ECall0OutputArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
struct ECall0OutputLayout {
  arm0: ECall0OutputArm0Layout,
  arm1: ECall0OutputArm1Layout,
  arm2: ECall0OutputArm2Layout,
  arm3: ECall0OutputArm3Layout,
  arm4: ECall0OutputArm4Layout,
  arm5: ECall0OutputArm5Layout,
  arm6: ECall0OutputArm6Layout,
  arm7: ECall0OutputArm7Layout,
}
struct BoundLayout_ECall0OutputLayout {
  lyt: ECall0OutputLayout,
  buf: u32,
}
fn lookup_ECall0OutputLayout_arm0(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm0Layout {
  return BoundLayout_ECall0OutputArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_ECall0OutputLayout_arm1(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm1Layout {
  return BoundLayout_ECall0OutputArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_ECall0OutputLayout_arm2(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm2Layout {
  return BoundLayout_ECall0OutputArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_ECall0OutputLayout_arm3(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm3Layout {
  return BoundLayout_ECall0OutputArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_ECall0OutputLayout_arm4(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm4Layout {
  return BoundLayout_ECall0OutputArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_ECall0OutputLayout_arm5(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm5Layout {
  return BoundLayout_ECall0OutputArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_ECall0OutputLayout_arm6(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm6Layout {
  return BoundLayout_ECall0OutputArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_ECall0OutputLayout_arm7(b: BoundLayout_ECall0OutputLayout) -> BoundLayout_ECall0OutputArm7Layout {
  return BoundLayout_ECall0OutputArm7Layout(b.lyt.arm7, b.buf);
}
struct ECall0Layout {
  s0: NondetRegLayout,
  s1: NondetRegLayout,
  s2: NondetRegLayout,
  _0: DoCycleTableLayout,
  pcAddr: AddrDecomposeBitsLayout,
  _arguments_ECall0Output: _Arguments_ECall0OutputLayout,
  output: ECall0OutputLayout,
  isSuspend: IsZeroLayout,
  isDecode: IsZeroLayout,
  isP2Entry: IsZeroLayout,
  isShaEcall: IsZeroLayout,
  isBigIntEcall: IsZeroLayout,
  addPC: NormalizeU32Layout,
}
struct BoundLayout_ECall0Layout {
  lyt: ECall0Layout,
  buf: u32,
}
fn lookup_ECall0Layout_s0(b: BoundLayout_ECall0Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.s0, b.buf);
}
fn lookup_ECall0Layout_s1(b: BoundLayout_ECall0Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.s1, b.buf);
}
fn lookup_ECall0Layout_s2(b: BoundLayout_ECall0Layout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.s2, b.buf);
}
fn lookup_ECall0Layout__0(b: BoundLayout_ECall0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_ECall0Layout_pcAddr(b: BoundLayout_ECall0Layout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt.pcAddr, b.buf);
}
fn lookup_ECall0Layout__arguments_ECall0Output(b: BoundLayout_ECall0Layout) -> BoundLayout__Arguments_ECall0OutputLayout {
  return BoundLayout__Arguments_ECall0OutputLayout(b.lyt._arguments_ECall0Output, b.buf);
}
fn lookup_ECall0Layout_output(b: BoundLayout_ECall0Layout) -> BoundLayout_ECall0OutputLayout {
  return BoundLayout_ECall0OutputLayout(b.lyt.output, b.buf);
}
fn lookup_ECall0Layout_isSuspend(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isSuspend, b.buf);
}
fn lookup_ECall0Layout_isDecode(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isDecode, b.buf);
}
fn lookup_ECall0Layout_isP2Entry(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isP2Entry, b.buf);
}
fn lookup_ECall0Layout_isShaEcall(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isShaEcall, b.buf);
}
fn lookup_ECall0Layout_isBigIntEcall(b: BoundLayout_ECall0Layout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isBigIntEcall, b.buf);
}
fn lookup_ECall0Layout_addPC(b: BoundLayout_ECall0Layout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.addPC, b.buf);
}
alias NondetRegLayout24LayoutArray = array<NondetRegLayout, 24>;
struct BoundLayout_NondetRegLayout24LayoutArray {
  lyt: NondetRegLayout24LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout24LayoutArray(b: BoundLayout_NondetRegLayout24LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct PoseidonStateLayout {
  hasState: NondetRegLayout,
  stateAddr: NondetRegLayout,
  bufOutAddr: NondetRegLayout,
  isElem: NondetRegLayout,
  checkOut: NondetRegLayout,
  loadTxType: NondetRegLayout,
  nextState: NondetRegLayout,
  subState: NondetRegLayout,
  bufInAddr: NondetRegLayout,
  count: NondetRegLayout,
  mode: NondetRegLayout,
  inner: NondetRegLayout24LayoutArray,
  zcheck: NondetExtRegLayout,
}
struct BoundLayout_PoseidonStateLayout {
  lyt: PoseidonStateLayout,
  buf: u32,
}
fn lookup_PoseidonStateLayout_hasState(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.hasState, b.buf);
}
fn lookup_PoseidonStateLayout_stateAddr(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.stateAddr, b.buf);
}
fn lookup_PoseidonStateLayout_bufOutAddr(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bufOutAddr, b.buf);
}
fn lookup_PoseidonStateLayout_isElem(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isElem, b.buf);
}
fn lookup_PoseidonStateLayout_checkOut(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.checkOut, b.buf);
}
fn lookup_PoseidonStateLayout_loadTxType(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.loadTxType, b.buf);
}
fn lookup_PoseidonStateLayout_nextState(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState, b.buf);
}
fn lookup_PoseidonStateLayout_subState(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.subState, b.buf);
}
fn lookup_PoseidonStateLayout_bufInAddr(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bufInAddr, b.buf);
}
fn lookup_PoseidonStateLayout_count(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_PoseidonStateLayout_mode(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.mode, b.buf);
}
fn lookup_PoseidonStateLayout_inner(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetRegLayout24LayoutArray {
  return BoundLayout_NondetRegLayout24LayoutArray(b.lyt.inner, b.buf);
}
fn lookup_PoseidonStateLayout_zcheck(b: BoundLayout_PoseidonStateLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.zcheck, b.buf);
}
alias ArgU16Layout24LayoutArray = array<ArgU16Layout, 24>;
struct BoundLayout_ArgU16Layout24LayoutArray {
  lyt: ArgU16Layout24LayoutArray,
  buf: u32,
}
fn subscript_ArgU16Layout24LayoutArray(b: BoundLayout_ArgU16Layout24LayoutArray, i: u32) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt[i], b.buf);
}
alias ArgU8Layout2LayoutArray = array<ArgU8Layout, 2>;
struct BoundLayout_ArgU8Layout2LayoutArray {
  lyt: ArgU8Layout2LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout2LayoutArray(b: BoundLayout_ArgU8Layout2LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_Poseidon0StateLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
  argU16: ArgU16Layout24LayoutArray,
  argU8: ArgU8Layout2LayoutArray,
}
struct BoundLayout__Arguments_Poseidon0StateLayout {
  lyt: _Arguments_Poseidon0StateLayout,
  buf: u32,
}
fn lookup__Arguments_Poseidon0StateLayout_memoryArg(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_Poseidon0StateLayout_cycleArg(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_Poseidon0StateLayout_argU16(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_ArgU16Layout24LayoutArray {
  return BoundLayout_ArgU16Layout24LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_Poseidon0StateLayout_argU8(b: BoundLayout__Arguments_Poseidon0StateLayout) -> BoundLayout_ArgU8Layout2LayoutArray {
  return BoundLayout_ArgU8Layout2LayoutArray(b.lyt.argU8, b.buf);
}
struct PoseidonEntry_SuperArm0Layout {
  _super: PoseidonStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
}
struct BoundLayout_PoseidonEntry_SuperArm0Layout {
  lyt: PoseidonEntry_SuperArm0Layout,
  buf: u32,
}
fn lookup_PoseidonEntry_SuperArm0Layout__super(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra0(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra1(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra2(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra3(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra4(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra5(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra6(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra7(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra8(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra9(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra10(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_PoseidonEntry_SuperArm0Layout__extra11(b: BoundLayout_PoseidonEntry_SuperArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
struct ReadAddrLayout {
  addr32: MemoryReadLayout,
}
struct BoundLayout_ReadAddrLayout {
  lyt: ReadAddrLayout,
  buf: u32,
}
fn lookup_ReadAddrLayout_addr32(b: BoundLayout_ReadAddrLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.addr32, b.buf);
}
struct PoseidonEcallLayout {
  _super: PoseidonStateLayout,
  stateAddr: ReadAddrLayout,
  bufInAddr: ReadAddrLayout,
  bufOutAddr: ReadAddrLayout,
  bitsAndCount: MemoryReadLayout,
  _0: IsZeroLayout,
  isElem: NondetRegLayout,
  checkOut: NondetRegLayout,
  countZero: IsZeroLayout,
}
struct BoundLayout_PoseidonEcallLayout {
  lyt: PoseidonEcallLayout,
  buf: u32,
}
fn lookup_PoseidonEcallLayout__super(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEcallLayout_stateAddr(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.stateAddr, b.buf);
}
fn lookup_PoseidonEcallLayout_bufInAddr(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.bufInAddr, b.buf);
}
fn lookup_PoseidonEcallLayout_bufOutAddr(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.bufOutAddr, b.buf);
}
fn lookup_PoseidonEcallLayout_bitsAndCount(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.bitsAndCount, b.buf);
}
fn lookup_PoseidonEcallLayout__0(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
fn lookup_PoseidonEcallLayout_isElem(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isElem, b.buf);
}
fn lookup_PoseidonEcallLayout_checkOut(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.checkOut, b.buf);
}
fn lookup_PoseidonEcallLayout_countZero(b: BoundLayout_PoseidonEcallLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.countZero, b.buf);
}
struct PoseidonEntry_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonEntry_SuperArm0Layout,
  arm1: PoseidonEcallLayout,
}
struct BoundLayout_PoseidonEntry_SuperLayout {
  lyt: PoseidonEntry_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonEntry_SuperLayout__super(b: BoundLayout_PoseidonEntry_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEntry_SuperLayout_arm0(b: BoundLayout_PoseidonEntry_SuperLayout) -> BoundLayout_PoseidonEntry_SuperArm0Layout {
  return BoundLayout_PoseidonEntry_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonEntry_SuperLayout_arm1(b: BoundLayout_PoseidonEntry_SuperLayout) -> BoundLayout_PoseidonEcallLayout {
  return BoundLayout_PoseidonEcallLayout(b.lyt.arm1, b.buf);
}
struct _Arguments_PoseidonEntry_SuperLayout {
  memoryArg: MemoryArgLayout8LayoutArray,
  cycleArg: CycleArgLayout4LayoutArray,
}
struct BoundLayout__Arguments_PoseidonEntry_SuperLayout {
  lyt: _Arguments_PoseidonEntry_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_PoseidonEntry_SuperLayout_memoryArg(b: BoundLayout__Arguments_PoseidonEntry_SuperLayout) -> BoundLayout_MemoryArgLayout8LayoutArray {
  return BoundLayout_MemoryArgLayout8LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_PoseidonEntry_SuperLayout_cycleArg(b: BoundLayout__Arguments_PoseidonEntry_SuperLayout) -> BoundLayout_CycleArgLayout4LayoutArray {
  return BoundLayout_CycleArgLayout4LayoutArray(b.lyt.cycleArg, b.buf);
}
struct PoseidonEntryLayout {
  _super: PoseidonEntry_SuperLayout,
  pcZero: IsZeroLayout,
  _arguments_PoseidonEntry_Super: _Arguments_PoseidonEntry_SuperLayout,
}
struct BoundLayout_PoseidonEntryLayout {
  lyt: PoseidonEntryLayout,
  buf: u32,
}
fn lookup_PoseidonEntryLayout__super(b: BoundLayout_PoseidonEntryLayout) -> BoundLayout_PoseidonEntry_SuperLayout {
  return BoundLayout_PoseidonEntry_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonEntryLayout_pcZero(b: BoundLayout_PoseidonEntryLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.pcZero, b.buf);
}
fn lookup_PoseidonEntryLayout__arguments_PoseidonEntry_Super(b: BoundLayout_PoseidonEntryLayout) -> BoundLayout__Arguments_PoseidonEntry_SuperLayout {
  return BoundLayout__Arguments_PoseidonEntry_SuperLayout(b.lyt._arguments_PoseidonEntry_Super, b.buf);
}
struct Poseidon0StateArm0Layout {
  _super: PoseidonEntryLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU8Layout,
  _extra37: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm0Layout {
  lyt: Poseidon0StateArm0Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm0Layout__super(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_PoseidonEntryLayout {
  return BoundLayout_PoseidonEntryLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra0(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra1(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra2(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra3(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra4(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra5(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra6(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra7(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra8(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra9(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra10(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra11(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra12(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra13(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra14(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra15(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra16(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra17(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra18(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra19(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra20(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra21(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra22(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra23(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra24(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra25(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra26(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra27(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra28(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra29(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra30(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra31(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra32(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra33(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra34(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra35(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra36(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm0Layout__extra37(b: BoundLayout_Poseidon0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra37, b.buf);
}
struct ReadElemLayout {
  elem32: MemoryReadLayout,
}
struct BoundLayout_ReadElemLayout {
  lyt: ReadElemLayout,
  buf: u32,
}
fn lookup_ReadElemLayout_elem32(b: BoundLayout_ReadElemLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.elem32, b.buf);
}
alias ReadElemLayout8LayoutArray = array<ReadElemLayout, 8>;
struct BoundLayout_ReadElemLayout8LayoutArray {
  lyt: ReadElemLayout8LayoutArray,
  buf: u32,
}
fn subscript_ReadElemLayout8LayoutArray(b: BoundLayout_ReadElemLayout8LayoutArray, i: u32) -> BoundLayout_ReadElemLayout {
  return BoundLayout_ReadElemLayout(b.lyt[i], b.buf);
}
struct PoseidonLoadStateLayout {
  _super: PoseidonStateLayout,
  loadList: ReadElemLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadStateLayout {
  lyt: PoseidonLoadStateLayout,
  buf: u32,
}
fn lookup_PoseidonLoadStateLayout__super(b: BoundLayout_PoseidonLoadStateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadStateLayout_loadList(b: BoundLayout_PoseidonLoadStateLayout) -> BoundLayout_ReadElemLayout8LayoutArray {
  return BoundLayout_ReadElemLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct Poseidon0StateArm1Layout {
  _super: PoseidonLoadStateLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm1Layout {
  lyt: Poseidon0StateArm1Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm1Layout__super(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_PoseidonLoadStateLayout {
  return BoundLayout_PoseidonLoadStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra0(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra1(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra2(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra3(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra4(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra5(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra6(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra7(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra8(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra9(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra10(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra11(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra12(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra13(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra14(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra15(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra16(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra17(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra18(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra19(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra20(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra21(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra22(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra23(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra24(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm1Layout__extra25(b: BoundLayout_Poseidon0StateArm1Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
alias NondetRegLayout3LayoutArray = array<NondetRegLayout, 3>;
struct BoundLayout_NondetRegLayout3LayoutArray {
  lyt: NondetRegLayout3LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout3LayoutArray(b: BoundLayout_NondetRegLayout3LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct OneHot_3_Layout {
  _super: NondetRegLayout3LayoutArray,
}
struct BoundLayout_OneHot_3_Layout {
  lyt: OneHot_3_Layout,
  buf: u32,
}
fn lookup_OneHot_3_Layout__super(b: BoundLayout_OneHot_3_Layout) -> BoundLayout_NondetRegLayout3LayoutArray {
  return BoundLayout_NondetRegLayout3LayoutArray(b.lyt._super, b.buf);
}
struct MemoryGet_SuperArm1Layout {
  _super: MemoryPageInLayout,
  _extra0: CycleArgLayout,
}
struct BoundLayout_MemoryGet_SuperArm1Layout {
  lyt: MemoryGet_SuperArm1Layout,
  buf: u32,
}
fn lookup_MemoryGet_SuperArm1Layout__super(b: BoundLayout_MemoryGet_SuperArm1Layout) -> BoundLayout_MemoryPageInLayout {
  return BoundLayout_MemoryPageInLayout(b.lyt._super, b.buf);
}
fn lookup_MemoryGet_SuperArm1Layout__extra0(b: BoundLayout_MemoryGet_SuperArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra0, b.buf);
}
struct MemoryGet_SuperLayout {
  arm0: MemoryReadLayout,
  arm1: MemoryGet_SuperArm1Layout,
  arm2: MemoryPageOutLayout,
}
struct BoundLayout_MemoryGet_SuperLayout {
  lyt: MemoryGet_SuperLayout,
  buf: u32,
}
fn lookup_MemoryGet_SuperLayout_arm0(b: BoundLayout_MemoryGet_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.arm0, b.buf);
}
fn lookup_MemoryGet_SuperLayout_arm1(b: BoundLayout_MemoryGet_SuperLayout) -> BoundLayout_MemoryGet_SuperArm1Layout {
  return BoundLayout_MemoryGet_SuperArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_MemoryGet_SuperLayout_arm2(b: BoundLayout_MemoryGet_SuperLayout) -> BoundLayout_MemoryPageOutLayout {
  return BoundLayout_MemoryPageOutLayout(b.lyt.arm2, b.buf);
}
alias MemoryArgLayout2LayoutArray = array<MemoryArgLayout, 2>;
struct BoundLayout_MemoryArgLayout2LayoutArray {
  lyt: MemoryArgLayout2LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout2LayoutArray(b: BoundLayout_MemoryArgLayout2LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_MemoryGet_SuperLayout {
  memoryArg: MemoryArgLayout2LayoutArray,
  cycleArg: CycleArgLayout1LayoutArray,
}
struct BoundLayout__Arguments_MemoryGet_SuperLayout {
  lyt: _Arguments_MemoryGet_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_MemoryGet_SuperLayout_memoryArg(b: BoundLayout__Arguments_MemoryGet_SuperLayout) -> BoundLayout_MemoryArgLayout2LayoutArray {
  return BoundLayout_MemoryArgLayout2LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_MemoryGet_SuperLayout_cycleArg(b: BoundLayout__Arguments_MemoryGet_SuperLayout) -> BoundLayout_CycleArgLayout1LayoutArray {
  return BoundLayout_CycleArgLayout1LayoutArray(b.lyt.cycleArg, b.buf);
}
struct MemoryGetLayout {
  _super: MemoryGet_SuperLayout,
  _arguments_MemoryGet_Super: _Arguments_MemoryGet_SuperLayout,
}
struct BoundLayout_MemoryGetLayout {
  lyt: MemoryGetLayout,
  buf: u32,
}
fn lookup_MemoryGetLayout__super(b: BoundLayout_MemoryGetLayout) -> BoundLayout_MemoryGet_SuperLayout {
  return BoundLayout_MemoryGet_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_MemoryGetLayout__arguments_MemoryGet_Super(b: BoundLayout_MemoryGetLayout) -> BoundLayout__Arguments_MemoryGet_SuperLayout {
  return BoundLayout__Arguments_MemoryGet_SuperLayout(b.lyt._arguments_MemoryGet_Super, b.buf);
}
alias MemoryGetLayout8LayoutArray = array<MemoryGetLayout, 8>;
struct BoundLayout_MemoryGetLayout8LayoutArray {
  lyt: MemoryGetLayout8LayoutArray,
  buf: u32,
}
fn subscript_MemoryGetLayout8LayoutArray(b: BoundLayout_MemoryGetLayout8LayoutArray, i: u32) -> BoundLayout_MemoryGetLayout {
  return BoundLayout_MemoryGetLayout(b.lyt[i], b.buf);
}
struct PoseidonLoadInShortLayout {
  _super: PoseidonStateLayout,
  txType: OneHot_3_Layout,
  loadList: MemoryGetLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadInShortLayout {
  lyt: PoseidonLoadInShortLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInShortLayout__super(b: BoundLayout_PoseidonLoadInShortLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInShortLayout_txType(b: BoundLayout_PoseidonLoadInShortLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.txType, b.buf);
}
fn lookup_PoseidonLoadInShortLayout_loadList(b: BoundLayout_PoseidonLoadInShortLayout) -> BoundLayout_MemoryGetLayout8LayoutArray {
  return BoundLayout_MemoryGetLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct PoseidonLoadInLowLayout {
  _super: PoseidonStateLayout,
  txType: OneHot_3_Layout,
  loadList: MemoryGetLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadInLowLayout {
  lyt: PoseidonLoadInLowLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInLowLayout__super(b: BoundLayout_PoseidonLoadInLowLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInLowLayout_txType(b: BoundLayout_PoseidonLoadInLowLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.txType, b.buf);
}
fn lookup_PoseidonLoadInLowLayout_loadList(b: BoundLayout_PoseidonLoadInLowLayout) -> BoundLayout_MemoryGetLayout8LayoutArray {
  return BoundLayout_MemoryGetLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct PoseidonLoadInHighLayout {
  _super: PoseidonStateLayout,
  txType: OneHot_3_Layout,
  loadList: MemoryGetLayout8LayoutArray,
}
struct BoundLayout_PoseidonLoadInHighLayout {
  lyt: PoseidonLoadInHighLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInHighLayout__super(b: BoundLayout_PoseidonLoadInHighLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInHighLayout_txType(b: BoundLayout_PoseidonLoadInHighLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.txType, b.buf);
}
fn lookup_PoseidonLoadInHighLayout_loadList(b: BoundLayout_PoseidonLoadInHighLayout) -> BoundLayout_MemoryGetLayout8LayoutArray {
  return BoundLayout_MemoryGetLayout8LayoutArray(b.lyt.loadList, b.buf);
}
struct PoseidonLoadIn_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonLoadInShortLayout,
  arm1: PoseidonLoadInLowLayout,
  arm2: PoseidonLoadInHighLayout,
}
struct BoundLayout_PoseidonLoadIn_SuperLayout {
  lyt: PoseidonLoadIn_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonLoadIn_SuperLayout__super(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadIn_SuperLayout_arm0(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonLoadInShortLayout {
  return BoundLayout_PoseidonLoadInShortLayout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonLoadIn_SuperLayout_arm1(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonLoadInLowLayout {
  return BoundLayout_PoseidonLoadInLowLayout(b.lyt.arm1, b.buf);
}
fn lookup_PoseidonLoadIn_SuperLayout_arm2(b: BoundLayout_PoseidonLoadIn_SuperLayout) -> BoundLayout_PoseidonLoadInHighLayout {
  return BoundLayout_PoseidonLoadInHighLayout(b.lyt.arm2, b.buf);
}
struct _Arguments_PoseidonLoadIn_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
}
struct BoundLayout__Arguments_PoseidonLoadIn_SuperLayout {
  lyt: _Arguments_PoseidonLoadIn_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_PoseidonLoadIn_SuperLayout_memoryArg(b: BoundLayout__Arguments_PoseidonLoadIn_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_PoseidonLoadIn_SuperLayout_cycleArg(b: BoundLayout__Arguments_PoseidonLoadIn_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
struct PoseidonLoadInLayout {
  _super: PoseidonLoadIn_SuperLayout,
  _0: OneHot_3_Layout,
  _arguments_PoseidonLoadIn_Super: _Arguments_PoseidonLoadIn_SuperLayout,
}
struct BoundLayout_PoseidonLoadInLayout {
  lyt: PoseidonLoadInLayout,
  buf: u32,
}
fn lookup_PoseidonLoadInLayout__super(b: BoundLayout_PoseidonLoadInLayout) -> BoundLayout_PoseidonLoadIn_SuperLayout {
  return BoundLayout_PoseidonLoadIn_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonLoadInLayout__0(b: BoundLayout_PoseidonLoadInLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt._0, b.buf);
}
fn lookup_PoseidonLoadInLayout__arguments_PoseidonLoadIn_Super(b: BoundLayout_PoseidonLoadInLayout) -> BoundLayout__Arguments_PoseidonLoadIn_SuperLayout {
  return BoundLayout__Arguments_PoseidonLoadIn_SuperLayout(b.lyt._arguments_PoseidonLoadIn_Super, b.buf);
}
struct Poseidon0StateArm2Layout {
  _super: PoseidonLoadInLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm2Layout {
  lyt: Poseidon0StateArm2Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm2Layout__super(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_PoseidonLoadInLayout {
  return BoundLayout_PoseidonLoadInLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra0(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra1(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra2(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra3(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra4(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra5(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra6(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra7(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra8(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra9(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra10(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra11(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra12(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra13(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra14(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra15(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra16(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra17(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra18(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra19(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra20(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra21(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra22(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra23(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra24(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm2Layout__extra25(b: BoundLayout_Poseidon0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
struct Poseidon0StateArm3Layout {
  _super: PoseidonStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
  _extra40: ArgU16Layout,
  _extra41: ArgU16Layout,
  _extra42: ArgU16Layout,
  _extra43: ArgU16Layout,
  _extra44: ArgU16Layout,
  _extra45: ArgU16Layout,
  _extra46: ArgU16Layout,
  _extra47: ArgU16Layout,
  _extra48: ArgU8Layout,
  _extra49: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm3Layout {
  lyt: Poseidon0StateArm3Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm3Layout__super(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra0(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra1(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra2(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra3(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra4(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra5(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra6(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra7(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra8(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra9(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra10(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra11(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra12(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra13(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra14(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra15(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra16(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra17(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra18(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra19(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra20(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra21(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra22(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra23(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra24(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra25(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra26(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra27(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra28(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra29(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra30(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra31(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra32(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra33(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra34(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra35(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra36(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra37(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra38(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra39(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra40(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra40, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra41(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra41, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra42(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra42, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra43(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra43, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra44(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra44, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra45(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra45, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra46(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra46, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra47(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra47, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra48(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra48, b.buf);
}
fn lookup_Poseidon0StateArm3Layout__extra49(b: BoundLayout_Poseidon0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra49, b.buf);
}
struct Poseidon0StateArm4Layout {
  _super: PoseidonStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
  _extra40: ArgU16Layout,
  _extra41: ArgU16Layout,
  _extra42: ArgU16Layout,
  _extra43: ArgU16Layout,
  _extra44: ArgU16Layout,
  _extra45: ArgU16Layout,
  _extra46: ArgU16Layout,
  _extra47: ArgU16Layout,
  _extra48: ArgU8Layout,
  _extra49: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm4Layout {
  lyt: Poseidon0StateArm4Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm4Layout__super(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra0(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra1(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra2(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra3(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra4(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra5(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra6(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra7(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra8(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra9(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra10(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra11(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra12(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra13(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra14(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra15(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra16(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra17(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra18(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra19(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra20(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra21(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra22(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra23(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra24(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra25(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra26(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra27(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra28(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra29(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra30(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra31(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra32(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra33(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra34(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra35(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra36(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra37(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra38(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra39(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra40(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra40, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra41(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra41, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra42(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra42, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra43(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra43, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra44(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra44, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra45(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra45, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra46(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra46, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra47(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra47, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra48(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra48, b.buf);
}
fn lookup_Poseidon0StateArm4Layout__extra49(b: BoundLayout_Poseidon0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra49, b.buf);
}
struct PoseidonCheckOut__0_SuperLayout {
  goal: ReadElemLayout,
}
struct BoundLayout_PoseidonCheckOut__0_SuperLayout {
  lyt: PoseidonCheckOut__0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonCheckOut__0_SuperLayout_goal(b: BoundLayout_PoseidonCheckOut__0_SuperLayout) -> BoundLayout_ReadElemLayout {
  return BoundLayout_ReadElemLayout(b.lyt.goal, b.buf);
}
alias PoseidonCheckOut__0_SuperLayout8LayoutArray = array<PoseidonCheckOut__0_SuperLayout, 8>;
struct BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray {
  lyt: PoseidonCheckOut__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_PoseidonCheckOut__0_SuperLayout8LayoutArray(b: BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_PoseidonCheckOut__0_SuperLayout {
  return BoundLayout_PoseidonCheckOut__0_SuperLayout(b.lyt[i], b.buf);
}
struct PoseidonCheckOutLayout {
  _super: PoseidonStateLayout,
  _1: PoseidonCheckOut__0_SuperLayout8LayoutArray,
  isNormal: IsZeroLayout,
}
struct BoundLayout_PoseidonCheckOutLayout {
  lyt: PoseidonCheckOutLayout,
  buf: u32,
}
fn lookup_PoseidonCheckOutLayout__super(b: BoundLayout_PoseidonCheckOutLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonCheckOutLayout__1(b: BoundLayout_PoseidonCheckOutLayout) -> BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray {
  return BoundLayout_PoseidonCheckOut__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
fn lookup_PoseidonCheckOutLayout_isNormal(b: BoundLayout_PoseidonCheckOutLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isNormal, b.buf);
}
struct PoseidonDoOut_SuperArm0Layout {
  _super: PoseidonCheckOutLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: ArgU16Layout,
  _extra3: ArgU16Layout,
  _extra4: ArgU16Layout,
  _extra5: ArgU16Layout,
  _extra6: ArgU16Layout,
  _extra7: ArgU16Layout,
  _extra8: ArgU16Layout,
  _extra9: ArgU16Layout,
  _extra10: ArgU16Layout,
  _extra11: ArgU16Layout,
  _extra12: ArgU16Layout,
  _extra13: ArgU16Layout,
  _extra14: ArgU16Layout,
  _extra15: ArgU16Layout,
  _extra16: ArgU16Layout,
  _extra17: ArgU16Layout,
  _extra18: ArgU16Layout,
  _extra19: ArgU16Layout,
  _extra20: ArgU16Layout,
  _extra21: ArgU16Layout,
  _extra22: ArgU16Layout,
  _extra23: ArgU16Layout,
}
struct BoundLayout_PoseidonDoOut_SuperArm0Layout {
  lyt: PoseidonDoOut_SuperArm0Layout,
  buf: u32,
}
fn lookup_PoseidonDoOut_SuperArm0Layout__super(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_PoseidonCheckOutLayout {
  return BoundLayout_PoseidonCheckOutLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra0(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra1(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra2(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra2, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra3(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra3, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra4(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra4, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra5(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra5, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra6(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra6, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra7(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra7, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra8(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra8, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra9(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra9, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra10(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra10, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra11(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra11, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra12(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra12, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra13(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra13, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra14(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra14, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra15(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra15, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra16(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra16, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra17(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra17, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra18(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra18, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra19(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra19, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra20(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra20, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra21(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra21, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra22(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra22, b.buf);
}
fn lookup_PoseidonDoOut_SuperArm0Layout__extra23(b: BoundLayout_PoseidonDoOut_SuperArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra23, b.buf);
}
struct _Arguments_FieldToWord__0Layout {
  argU16: ArgU16Layout1LayoutArray,
}
struct BoundLayout__Arguments_FieldToWord__0Layout {
  lyt: _Arguments_FieldToWord__0Layout,
  buf: u32,
}
fn lookup__Arguments_FieldToWord__0Layout_argU16(b: BoundLayout__Arguments_FieldToWord__0Layout) -> BoundLayout_ArgU16Layout1LayoutArray {
  return BoundLayout_ArgU16Layout1LayoutArray(b.lyt.argU16, b.buf);
}
struct FieldToWord__0Arm0_SuperLayout {
  _0: NondetU16RegLayout,
}
struct BoundLayout_FieldToWord__0Arm0_SuperLayout {
  lyt: FieldToWord__0Arm0_SuperLayout,
  buf: u32,
}
fn lookup_FieldToWord__0Arm0_SuperLayout__0(b: BoundLayout_FieldToWord__0Arm0_SuperLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt._0, b.buf);
}
struct FieldToWord__0Arm1_SuperLayout {
  _0: NondetU16RegLayout,
}
struct BoundLayout_FieldToWord__0Arm1_SuperLayout {
  lyt: FieldToWord__0Arm1_SuperLayout,
  buf: u32,
}
fn lookup_FieldToWord__0Arm1_SuperLayout__0(b: BoundLayout_FieldToWord__0Arm1_SuperLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt._0, b.buf);
}
struct FieldToWord__0Layout {
  arm0: FieldToWord__0Arm0_SuperLayout,
  arm1: FieldToWord__0Arm1_SuperLayout,
}
struct BoundLayout_FieldToWord__0Layout {
  lyt: FieldToWord__0Layout,
  buf: u32,
}
fn lookup_FieldToWord__0Layout_arm0(b: BoundLayout_FieldToWord__0Layout) -> BoundLayout_FieldToWord__0Arm0_SuperLayout {
  return BoundLayout_FieldToWord__0Arm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_FieldToWord__0Layout_arm1(b: BoundLayout_FieldToWord__0Layout) -> BoundLayout_FieldToWord__0Arm1_SuperLayout {
  return BoundLayout_FieldToWord__0Arm1_SuperLayout(b.lyt.arm1, b.buf);
}
struct FieldToWordLayout {
  low: NondetU16RegLayout,
  high: NondetU16RegLayout,
  lowIsZero: NondetRegLayout,
  _arguments_FieldToWord__0: _Arguments_FieldToWord__0Layout,
  _2: FieldToWord__0Layout,
}
struct BoundLayout_FieldToWordLayout {
  lyt: FieldToWordLayout,
  buf: u32,
}
fn lookup_FieldToWordLayout_low(b: BoundLayout_FieldToWordLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low, b.buf);
}
fn lookup_FieldToWordLayout_high(b: BoundLayout_FieldToWordLayout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.high, b.buf);
}
fn lookup_FieldToWordLayout_lowIsZero(b: BoundLayout_FieldToWordLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.lowIsZero, b.buf);
}
fn lookup_FieldToWordLayout__arguments_FieldToWord__0(b: BoundLayout_FieldToWordLayout) -> BoundLayout__Arguments_FieldToWord__0Layout {
  return BoundLayout__Arguments_FieldToWord__0Layout(b.lyt._arguments_FieldToWord__0, b.buf);
}
fn lookup_FieldToWordLayout__2(b: BoundLayout_FieldToWordLayout) -> BoundLayout_FieldToWord__0Layout {
  return BoundLayout_FieldToWord__0Layout(b.lyt._2, b.buf);
}
struct PoseidonStoreOut__0_SuperLayout {
  ftw: FieldToWordLayout,
  mw: MemoryWriteLayout,
}
struct BoundLayout_PoseidonStoreOut__0_SuperLayout {
  lyt: PoseidonStoreOut__0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonStoreOut__0_SuperLayout_ftw(b: BoundLayout_PoseidonStoreOut__0_SuperLayout) -> BoundLayout_FieldToWordLayout {
  return BoundLayout_FieldToWordLayout(b.lyt.ftw, b.buf);
}
fn lookup_PoseidonStoreOut__0_SuperLayout_mw(b: BoundLayout_PoseidonStoreOut__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt.mw, b.buf);
}
alias PoseidonStoreOut__0_SuperLayout8LayoutArray = array<PoseidonStoreOut__0_SuperLayout, 8>;
struct BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray {
  lyt: PoseidonStoreOut__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_PoseidonStoreOut__0_SuperLayout8LayoutArray(b: BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_PoseidonStoreOut__0_SuperLayout {
  return BoundLayout_PoseidonStoreOut__0_SuperLayout(b.lyt[i], b.buf);
}
struct PoseidonStoreOutLayout {
  _super: PoseidonStateLayout,
  _1: PoseidonStoreOut__0_SuperLayout8LayoutArray,
  isNormal: IsZeroLayout,
  extInv: NondetExtRegLayout,
}
struct BoundLayout_PoseidonStoreOutLayout {
  lyt: PoseidonStoreOutLayout,
  buf: u32,
}
fn lookup_PoseidonStoreOutLayout__super(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonStoreOutLayout__1(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray {
  return BoundLayout_PoseidonStoreOut__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
fn lookup_PoseidonStoreOutLayout_isNormal(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isNormal, b.buf);
}
fn lookup_PoseidonStoreOutLayout_extInv(b: BoundLayout_PoseidonStoreOutLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.extInv, b.buf);
}
struct PoseidonDoOut_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonDoOut_SuperArm0Layout,
  arm1: PoseidonStoreOutLayout,
}
struct BoundLayout_PoseidonDoOut_SuperLayout {
  lyt: PoseidonDoOut_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonDoOut_SuperLayout__super(b: BoundLayout_PoseidonDoOut_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonDoOut_SuperLayout_arm0(b: BoundLayout_PoseidonDoOut_SuperLayout) -> BoundLayout_PoseidonDoOut_SuperArm0Layout {
  return BoundLayout_PoseidonDoOut_SuperArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonDoOut_SuperLayout_arm1(b: BoundLayout_PoseidonDoOut_SuperLayout) -> BoundLayout_PoseidonStoreOutLayout {
  return BoundLayout_PoseidonStoreOutLayout(b.lyt.arm1, b.buf);
}
struct _Arguments_PoseidonDoOut_SuperLayout {
  memoryArg: MemoryArgLayout16LayoutArray,
  cycleArg: CycleArgLayout8LayoutArray,
  argU16: ArgU16Layout24LayoutArray,
}
struct BoundLayout__Arguments_PoseidonDoOut_SuperLayout {
  lyt: _Arguments_PoseidonDoOut_SuperLayout,
  buf: u32,
}
fn lookup__Arguments_PoseidonDoOut_SuperLayout_memoryArg(b: BoundLayout__Arguments_PoseidonDoOut_SuperLayout) -> BoundLayout_MemoryArgLayout16LayoutArray {
  return BoundLayout_MemoryArgLayout16LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_PoseidonDoOut_SuperLayout_cycleArg(b: BoundLayout__Arguments_PoseidonDoOut_SuperLayout) -> BoundLayout_CycleArgLayout8LayoutArray {
  return BoundLayout_CycleArgLayout8LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_PoseidonDoOut_SuperLayout_argU16(b: BoundLayout__Arguments_PoseidonDoOut_SuperLayout) -> BoundLayout_ArgU16Layout24LayoutArray {
  return BoundLayout_ArgU16Layout24LayoutArray(b.lyt.argU16, b.buf);
}
struct PoseidonDoOutLayout {
  _super: PoseidonDoOut_SuperLayout,
  _arguments_PoseidonDoOut_Super: _Arguments_PoseidonDoOut_SuperLayout,
}
struct BoundLayout_PoseidonDoOutLayout {
  lyt: PoseidonDoOutLayout,
  buf: u32,
}
fn lookup_PoseidonDoOutLayout__super(b: BoundLayout_PoseidonDoOutLayout) -> BoundLayout_PoseidonDoOut_SuperLayout {
  return BoundLayout_PoseidonDoOut_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonDoOutLayout__arguments_PoseidonDoOut_Super(b: BoundLayout_PoseidonDoOutLayout) -> BoundLayout__Arguments_PoseidonDoOut_SuperLayout {
  return BoundLayout__Arguments_PoseidonDoOut_SuperLayout(b.lyt._arguments_PoseidonDoOut_Super, b.buf);
}
struct Poseidon0StateArm5Layout {
  _super: PoseidonDoOutLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm5Layout {
  lyt: Poseidon0StateArm5Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm5Layout__super(b: BoundLayout_Poseidon0StateArm5Layout) -> BoundLayout_PoseidonDoOutLayout {
  return BoundLayout_PoseidonDoOutLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm5Layout__extra0(b: BoundLayout_Poseidon0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm5Layout__extra1(b: BoundLayout_Poseidon0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
struct PoseidonPaging_SuperLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonStateLayout,
  arm1: PoseidonStateLayout,
  arm2: PoseidonStateLayout,
  arm3: PoseidonStateLayout,
  arm4: PoseidonStateLayout,
  arm5: PoseidonStateLayout,
}
struct BoundLayout_PoseidonPaging_SuperLayout {
  lyt: PoseidonPaging_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonPaging_SuperLayout__super(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm0(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm1(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm1, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm2(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm2, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm3(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm3, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm4(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm4, b.buf);
}
fn lookup_PoseidonPaging_SuperLayout_arm5(b: BoundLayout_PoseidonPaging_SuperLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm5, b.buf);
}
struct IsU24Layout {
  low16: NondetU16RegLayout,
  _0: NondetU8RegLayout,
}
struct BoundLayout_IsU24Layout {
  lyt: IsU24Layout,
  buf: u32,
}
fn lookup_IsU24Layout_low16(b: BoundLayout_IsU24Layout) -> BoundLayout_NondetU16RegLayout {
  return BoundLayout_NondetU16RegLayout(b.lyt.low16, b.buf);
}
fn lookup_IsU24Layout__0(b: BoundLayout_IsU24Layout) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt._0, b.buf);
}
alias ArgU8Layout1LayoutArray = array<ArgU8Layout, 1>;
struct BoundLayout_ArgU8Layout1LayoutArray {
  lyt: ArgU8Layout1LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout1LayoutArray(b: BoundLayout_ArgU8Layout1LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_PoseidonPaging__1Layout {
  argU16: ArgU16Layout1LayoutArray,
  argU8: ArgU8Layout1LayoutArray,
}
struct BoundLayout__Arguments_PoseidonPaging__1Layout {
  lyt: _Arguments_PoseidonPaging__1Layout,
  buf: u32,
}
fn lookup__Arguments_PoseidonPaging__1Layout_argU16(b: BoundLayout__Arguments_PoseidonPaging__1Layout) -> BoundLayout_ArgU16Layout1LayoutArray {
  return BoundLayout_ArgU16Layout1LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_PoseidonPaging__1Layout_argU8(b: BoundLayout__Arguments_PoseidonPaging__1Layout) -> BoundLayout_ArgU8Layout1LayoutArray {
  return BoundLayout_ArgU8Layout1LayoutArray(b.lyt.argU8, b.buf);
}
struct PoseidonPaging__1Arm0_SuperLayout {
  _0: IsU24Layout,
}
struct BoundLayout_PoseidonPaging__1Arm0_SuperLayout {
  lyt: PoseidonPaging__1Arm0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonPaging__1Arm0_SuperLayout__0(b: BoundLayout_PoseidonPaging__1Arm0_SuperLayout) -> BoundLayout_IsU24Layout {
  return BoundLayout_IsU24Layout(b.lyt._0, b.buf);
}
struct PoseidonPaging__1Arm1_SuperLayout {
  _0: IsU24Layout,
}
struct BoundLayout_PoseidonPaging__1Arm1_SuperLayout {
  lyt: PoseidonPaging__1Arm1_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonPaging__1Arm1_SuperLayout__0(b: BoundLayout_PoseidonPaging__1Arm1_SuperLayout) -> BoundLayout_IsU24Layout {
  return BoundLayout_IsU24Layout(b.lyt._0, b.buf);
}
struct PoseidonPaging__1Layout {
  arm0: PoseidonPaging__1Arm0_SuperLayout,
  arm1: PoseidonPaging__1Arm1_SuperLayout,
}
struct BoundLayout_PoseidonPaging__1Layout {
  lyt: PoseidonPaging__1Layout,
  buf: u32,
}
fn lookup_PoseidonPaging__1Layout_arm0(b: BoundLayout_PoseidonPaging__1Layout) -> BoundLayout_PoseidonPaging__1Arm0_SuperLayout {
  return BoundLayout_PoseidonPaging__1Arm0_SuperLayout(b.lyt.arm0, b.buf);
}
fn lookup_PoseidonPaging__1Layout_arm1(b: BoundLayout_PoseidonPaging__1Layout) -> BoundLayout_PoseidonPaging__1Arm1_SuperLayout {
  return BoundLayout_PoseidonPaging__1Arm1_SuperLayout(b.lyt.arm1, b.buf);
}
struct PoseidonPagingLayout {
  _super: PoseidonPaging_SuperLayout,
  curIdx: NondetRegLayout,
  curMode: NondetRegLayout,
  modeSplit: OneHot_6_Layout,
  _0: IsU24Layout,
  _arguments_PoseidonPaging__1: _Arguments_PoseidonPaging__1Layout,
  _3: PoseidonPaging__1Layout,
  _4: NondetRegLayout,
}
struct BoundLayout_PoseidonPagingLayout {
  lyt: PoseidonPagingLayout,
  buf: u32,
}
fn lookup_PoseidonPagingLayout__super(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_PoseidonPaging_SuperLayout {
  return BoundLayout_PoseidonPaging_SuperLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonPagingLayout_curIdx(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.curIdx, b.buf);
}
fn lookup_PoseidonPagingLayout_curMode(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.curMode, b.buf);
}
fn lookup_PoseidonPagingLayout_modeSplit(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_OneHot_6_Layout {
  return BoundLayout_OneHot_6_Layout(b.lyt.modeSplit, b.buf);
}
fn lookup_PoseidonPagingLayout__0(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_IsU24Layout {
  return BoundLayout_IsU24Layout(b.lyt._0, b.buf);
}
fn lookup_PoseidonPagingLayout__arguments_PoseidonPaging__1(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout__Arguments_PoseidonPaging__1Layout {
  return BoundLayout__Arguments_PoseidonPaging__1Layout(b.lyt._arguments_PoseidonPaging__1, b.buf);
}
fn lookup_PoseidonPagingLayout__3(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_PoseidonPaging__1Layout {
  return BoundLayout_PoseidonPaging__1Layout(b.lyt._3, b.buf);
}
fn lookup_PoseidonPagingLayout__4(b: BoundLayout_PoseidonPagingLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._4, b.buf);
}
struct Poseidon0StateArm6Layout {
  _super: PoseidonPagingLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: MemoryArgLayout,
  _extra13: MemoryArgLayout,
  _extra14: MemoryArgLayout,
  _extra15: MemoryArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: CycleArgLayout,
  _extra19: CycleArgLayout,
  _extra20: CycleArgLayout,
  _extra21: CycleArgLayout,
  _extra22: CycleArgLayout,
  _extra23: CycleArgLayout,
  _extra24: ArgU16Layout,
  _extra25: ArgU16Layout,
  _extra26: ArgU16Layout,
  _extra27: ArgU16Layout,
  _extra28: ArgU16Layout,
  _extra29: ArgU16Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
  _extra34: ArgU16Layout,
  _extra35: ArgU16Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
  _extra40: ArgU16Layout,
  _extra41: ArgU16Layout,
  _extra42: ArgU16Layout,
  _extra43: ArgU16Layout,
  _extra44: ArgU16Layout,
  _extra45: ArgU16Layout,
}
struct BoundLayout_Poseidon0StateArm6Layout {
  lyt: Poseidon0StateArm6Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm6Layout__super(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_PoseidonPagingLayout {
  return BoundLayout_PoseidonPagingLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra0(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra1(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra2(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra3(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra4(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra5(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra6(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra7(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra8(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra9(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra10(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra11(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra12(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra13(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra14(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra15(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra16(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra17(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra18(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra18, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra19(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra19, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra20(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra20, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra21(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra21, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra22(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra22, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra23(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra23, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra24(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra24, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra25(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra25, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra26(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra26, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra27(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra27, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra28(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra28, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra29(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra29, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra30(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra31(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra32(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra33(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra34(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra34, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra35(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra35, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra36(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra37(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra38(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra39(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra40(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra40, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra41(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra41, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra42(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra42, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra43(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra43, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra44(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra44, b.buf);
}
fn lookup_Poseidon0StateArm6Layout__extra45(b: BoundLayout_Poseidon0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra45, b.buf);
}
struct PoseidonStoreState__0_SuperLayout {
  ftw: FieldToWordLayout,
  mw: MemoryWriteLayout,
}
struct BoundLayout_PoseidonStoreState__0_SuperLayout {
  lyt: PoseidonStoreState__0_SuperLayout,
  buf: u32,
}
fn lookup_PoseidonStoreState__0_SuperLayout_ftw(b: BoundLayout_PoseidonStoreState__0_SuperLayout) -> BoundLayout_FieldToWordLayout {
  return BoundLayout_FieldToWordLayout(b.lyt.ftw, b.buf);
}
fn lookup_PoseidonStoreState__0_SuperLayout_mw(b: BoundLayout_PoseidonStoreState__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt.mw, b.buf);
}
alias PoseidonStoreState__0_SuperLayout8LayoutArray = array<PoseidonStoreState__0_SuperLayout, 8>;
struct BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray {
  lyt: PoseidonStoreState__0_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_PoseidonStoreState__0_SuperLayout8LayoutArray(b: BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray, i: u32) -> BoundLayout_PoseidonStoreState__0_SuperLayout {
  return BoundLayout_PoseidonStoreState__0_SuperLayout(b.lyt[i], b.buf);
}
struct PoseidonStoreStateLayout {
  _super: PoseidonStateLayout,
  _1: PoseidonStoreState__0_SuperLayout8LayoutArray,
}
struct BoundLayout_PoseidonStoreStateLayout {
  lyt: PoseidonStoreStateLayout,
  buf: u32,
}
fn lookup_PoseidonStoreStateLayout__super(b: BoundLayout_PoseidonStoreStateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonStoreStateLayout__1(b: BoundLayout_PoseidonStoreStateLayout) -> BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray {
  return BoundLayout_PoseidonStoreState__0_SuperLayout8LayoutArray(b.lyt._1, b.buf);
}
struct Poseidon0StateArm7Layout {
  _super: PoseidonStoreStateLayout,
  _extra0: ArgU8Layout,
  _extra1: ArgU8Layout,
}
struct BoundLayout_Poseidon0StateArm7Layout {
  lyt: Poseidon0StateArm7Layout,
  buf: u32,
}
fn lookup_Poseidon0StateArm7Layout__super(b: BoundLayout_Poseidon0StateArm7Layout) -> BoundLayout_PoseidonStoreStateLayout {
  return BoundLayout_PoseidonStoreStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateArm7Layout__extra0(b: BoundLayout_Poseidon0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra0, b.buf);
}
fn lookup_Poseidon0StateArm7Layout__extra1(b: BoundLayout_Poseidon0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra1, b.buf);
}
struct Poseidon0StateLayout {
  _super: PoseidonStateLayout,
  arm0: Poseidon0StateArm0Layout,
  arm1: Poseidon0StateArm1Layout,
  arm2: Poseidon0StateArm2Layout,
  arm3: Poseidon0StateArm3Layout,
  arm4: Poseidon0StateArm4Layout,
  arm5: Poseidon0StateArm5Layout,
  arm6: Poseidon0StateArm6Layout,
  arm7: Poseidon0StateArm7Layout,
}
struct BoundLayout_Poseidon0StateLayout {
  lyt: Poseidon0StateLayout,
  buf: u32,
}
fn lookup_Poseidon0StateLayout__super(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon0StateLayout_arm0(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm0Layout {
  return BoundLayout_Poseidon0StateArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_Poseidon0StateLayout_arm1(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm1Layout {
  return BoundLayout_Poseidon0StateArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Poseidon0StateLayout_arm2(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm2Layout {
  return BoundLayout_Poseidon0StateArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Poseidon0StateLayout_arm3(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm3Layout {
  return BoundLayout_Poseidon0StateArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Poseidon0StateLayout_arm4(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm4Layout {
  return BoundLayout_Poseidon0StateArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Poseidon0StateLayout_arm5(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm5Layout {
  return BoundLayout_Poseidon0StateArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Poseidon0StateLayout_arm6(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm6Layout {
  return BoundLayout_Poseidon0StateArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Poseidon0StateLayout_arm7(b: BoundLayout_Poseidon0StateLayout) -> BoundLayout_Poseidon0StateArm7Layout {
  return BoundLayout_Poseidon0StateArm7Layout(b.lyt.arm7, b.buf);
}
struct Poseidon0Layout {
  _0: DoCycleTableLayout,
  state: PoseidonStateLayout,
  _arguments_Poseidon0State: _Arguments_Poseidon0StateLayout,
  stateRedef: Poseidon0StateLayout,
}
struct BoundLayout_Poseidon0Layout {
  lyt: Poseidon0Layout,
  buf: u32,
}
fn lookup_Poseidon0Layout__0(b: BoundLayout_Poseidon0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Poseidon0Layout_state(b: BoundLayout_Poseidon0Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.state, b.buf);
}
fn lookup_Poseidon0Layout__arguments_Poseidon0State(b: BoundLayout_Poseidon0Layout) -> BoundLayout__Arguments_Poseidon0StateLayout {
  return BoundLayout__Arguments_Poseidon0StateLayout(b.lyt._arguments_Poseidon0State, b.buf);
}
fn lookup_Poseidon0Layout_stateRedef(b: BoundLayout_Poseidon0Layout) -> BoundLayout_Poseidon0StateLayout {
  return BoundLayout_Poseidon0StateLayout(b.lyt.stateRedef, b.buf);
}
struct SBoxLayout {
  _super: NondetRegLayout,
  cubed: NondetRegLayout,
}
struct BoundLayout_SBoxLayout {
  lyt: SBoxLayout,
  buf: u32,
}
fn lookup_SBoxLayout__super(b: BoundLayout_SBoxLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_SBoxLayout_cubed(b: BoundLayout_SBoxLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cubed, b.buf);
}
alias SBoxLayout24LayoutArray = array<SBoxLayout, 24>;
struct BoundLayout_SBoxLayout24LayoutArray {
  lyt: SBoxLayout24LayoutArray,
  buf: u32,
}
fn subscript_SBoxLayout24LayoutArray(b: BoundLayout_SBoxLayout24LayoutArray, i: u32) -> BoundLayout_SBoxLayout {
  return BoundLayout_SBoxLayout(b.lyt[i], b.buf);
}
struct DoExtRoundLayout {
  _1: SBoxLayout24LayoutArray,
}
struct BoundLayout_DoExtRoundLayout {
  lyt: DoExtRoundLayout,
  buf: u32,
}
fn lookup_DoExtRoundLayout__1(b: BoundLayout_DoExtRoundLayout) -> BoundLayout_SBoxLayout24LayoutArray {
  return BoundLayout_SBoxLayout24LayoutArray(b.lyt._1, b.buf);
}
struct DoExtRoundByIdxLayout {
  _super: DoExtRoundLayout,
  idxHot: OneHot_8_Layout,
}
struct BoundLayout_DoExtRoundByIdxLayout {
  lyt: DoExtRoundByIdxLayout,
  buf: u32,
}
fn lookup_DoExtRoundByIdxLayout__super(b: BoundLayout_DoExtRoundByIdxLayout) -> BoundLayout_DoExtRoundLayout {
  return BoundLayout_DoExtRoundLayout(b.lyt._super, b.buf);
}
fn lookup_DoExtRoundByIdxLayout_idxHot(b: BoundLayout_DoExtRoundByIdxLayout) -> BoundLayout_OneHot_8_Layout {
  return BoundLayout_OneHot_8_Layout(b.lyt.idxHot, b.buf);
}
struct PoseidonExtRoundLayout {
  _super: PoseidonStateLayout,
  isRound3: IsZeroLayout,
  isRound7: IsZeroLayout,
  lastBlock: IsZeroLayout,
  nextInner: DoExtRoundByIdxLayout,
}
struct BoundLayout_PoseidonExtRoundLayout {
  lyt: PoseidonExtRoundLayout,
  buf: u32,
}
fn lookup_PoseidonExtRoundLayout__super(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonExtRoundLayout_isRound3(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isRound3, b.buf);
}
fn lookup_PoseidonExtRoundLayout_isRound7(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.isRound7, b.buf);
}
fn lookup_PoseidonExtRoundLayout_lastBlock(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastBlock, b.buf);
}
fn lookup_PoseidonExtRoundLayout_nextInner(b: BoundLayout_PoseidonExtRoundLayout) -> BoundLayout_DoExtRoundByIdxLayout {
  return BoundLayout_DoExtRoundByIdxLayout(b.lyt.nextInner, b.buf);
}
struct DoIntRoundLayout {
  sbox: SBoxLayout,
}
struct BoundLayout_DoIntRoundLayout {
  lyt: DoIntRoundLayout,
  buf: u32,
}
fn lookup_DoIntRoundLayout_sbox(b: BoundLayout_DoIntRoundLayout) -> BoundLayout_SBoxLayout {
  return BoundLayout_SBoxLayout(b.lyt.sbox, b.buf);
}
alias DoIntRoundLayout21LayoutArray = array<DoIntRoundLayout, 21>;
struct BoundLayout_DoIntRoundLayout21LayoutArray {
  lyt: DoIntRoundLayout21LayoutArray,
  buf: u32,
}
fn subscript_DoIntRoundLayout21LayoutArray(b: BoundLayout_DoIntRoundLayout21LayoutArray, i: u32) -> BoundLayout_DoIntRoundLayout {
  return BoundLayout_DoIntRoundLayout(b.lyt[i], b.buf);
}
struct DoIntRoundsLayout {
  _super: DoIntRoundLayout21LayoutArray,
}
struct BoundLayout_DoIntRoundsLayout {
  lyt: DoIntRoundsLayout,
  buf: u32,
}
fn lookup_DoIntRoundsLayout__super(b: BoundLayout_DoIntRoundsLayout) -> BoundLayout_DoIntRoundLayout21LayoutArray {
  return BoundLayout_DoIntRoundLayout21LayoutArray(b.lyt._super, b.buf);
}
struct PoseidonIntRoundsLayout {
  _super: PoseidonStateLayout,
  nextInner: DoIntRoundsLayout,
}
struct BoundLayout_PoseidonIntRoundsLayout {
  lyt: PoseidonIntRoundsLayout,
  buf: u32,
}
fn lookup_PoseidonIntRoundsLayout__super(b: BoundLayout_PoseidonIntRoundsLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_PoseidonIntRoundsLayout_nextInner(b: BoundLayout_PoseidonIntRoundsLayout) -> BoundLayout_DoIntRoundsLayout {
  return BoundLayout_DoIntRoundsLayout(b.lyt.nextInner, b.buf);
}
struct Poseidon1StateLayout {
  _super: PoseidonStateLayout,
  arm0: PoseidonExtRoundLayout,
  arm1: PoseidonIntRoundsLayout,
  arm2: PoseidonStateLayout,
  arm3: PoseidonStateLayout,
  arm4: PoseidonStateLayout,
  arm5: PoseidonStateLayout,
  arm6: PoseidonStateLayout,
  arm7: PoseidonStateLayout,
}
struct BoundLayout_Poseidon1StateLayout {
  lyt: Poseidon1StateLayout,
  buf: u32,
}
fn lookup_Poseidon1StateLayout__super(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt._super, b.buf);
}
fn lookup_Poseidon1StateLayout_arm0(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonExtRoundLayout {
  return BoundLayout_PoseidonExtRoundLayout(b.lyt.arm0, b.buf);
}
fn lookup_Poseidon1StateLayout_arm1(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonIntRoundsLayout {
  return BoundLayout_PoseidonIntRoundsLayout(b.lyt.arm1, b.buf);
}
fn lookup_Poseidon1StateLayout_arm2(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm2, b.buf);
}
fn lookup_Poseidon1StateLayout_arm3(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm3, b.buf);
}
fn lookup_Poseidon1StateLayout_arm4(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm4, b.buf);
}
fn lookup_Poseidon1StateLayout_arm5(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm5, b.buf);
}
fn lookup_Poseidon1StateLayout_arm6(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm6, b.buf);
}
fn lookup_Poseidon1StateLayout_arm7(b: BoundLayout_Poseidon1StateLayout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.arm7, b.buf);
}
struct Poseidon1Layout {
  _0: DoCycleTableLayout,
  state: PoseidonStateLayout,
  stateRedef: Poseidon1StateLayout,
}
struct BoundLayout_Poseidon1Layout {
  lyt: Poseidon1Layout,
  buf: u32,
}
fn lookup_Poseidon1Layout__0(b: BoundLayout_Poseidon1Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Poseidon1Layout_state(b: BoundLayout_Poseidon1Layout) -> BoundLayout_PoseidonStateLayout {
  return BoundLayout_PoseidonStateLayout(b.lyt.state, b.buf);
}
fn lookup_Poseidon1Layout_stateRedef(b: BoundLayout_Poseidon1Layout) -> BoundLayout_Poseidon1StateLayout {
  return BoundLayout_Poseidon1StateLayout(b.lyt.stateRedef, b.buf);
}
alias NondetRegLayout32LayoutArray = array<NondetRegLayout, 32>;
struct BoundLayout_NondetRegLayout32LayoutArray {
  lyt: NondetRegLayout32LayoutArray,
  buf: u32,
}
fn subscript_NondetRegLayout32LayoutArray(b: BoundLayout_NondetRegLayout32LayoutArray, i: u32) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt[i], b.buf);
}
struct ShaStateLayout {
  stateInAddr: NondetRegLayout,
  stateOutAddr: NondetRegLayout,
  dataAddr: NondetRegLayout,
  count: NondetRegLayout,
  kAddr: NondetRegLayout,
  round: NondetRegLayout,
  nextState: NondetRegLayout,
  a: NondetRegLayout32LayoutArray,
  e: NondetRegLayout32LayoutArray,
  w: NondetRegLayout32LayoutArray,
}
struct BoundLayout_ShaStateLayout {
  lyt: ShaStateLayout,
  buf: u32,
}
fn lookup_ShaStateLayout_stateInAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.stateInAddr, b.buf);
}
fn lookup_ShaStateLayout_stateOutAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.stateOutAddr, b.buf);
}
fn lookup_ShaStateLayout_dataAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.dataAddr, b.buf);
}
fn lookup_ShaStateLayout_count(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.count, b.buf);
}
fn lookup_ShaStateLayout_kAddr(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.kAddr, b.buf);
}
fn lookup_ShaStateLayout_round(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.round, b.buf);
}
fn lookup_ShaStateLayout_nextState(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState, b.buf);
}
fn lookup_ShaStateLayout_a(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.a, b.buf);
}
fn lookup_ShaStateLayout_e(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.e, b.buf);
}
fn lookup_ShaStateLayout_w(b: BoundLayout_ShaStateLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.w, b.buf);
}
alias MemoryArgLayout10LayoutArray = array<MemoryArgLayout, 10>;
struct BoundLayout_MemoryArgLayout10LayoutArray {
  lyt: MemoryArgLayout10LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout10LayoutArray(b: BoundLayout_MemoryArgLayout10LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout5LayoutArray = array<CycleArgLayout, 5>;
struct BoundLayout_CycleArgLayout5LayoutArray {
  lyt: CycleArgLayout5LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout5LayoutArray(b: BoundLayout_CycleArgLayout5LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
struct _Arguments_Sha0StateLayout {
  memoryArg: MemoryArgLayout10LayoutArray,
  cycleArg: CycleArgLayout5LayoutArray,
}
struct BoundLayout__Arguments_Sha0StateLayout {
  lyt: _Arguments_Sha0StateLayout,
  buf: u32,
}
fn lookup__Arguments_Sha0StateLayout_memoryArg(b: BoundLayout__Arguments_Sha0StateLayout) -> BoundLayout_MemoryArgLayout10LayoutArray {
  return BoundLayout_MemoryArgLayout10LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_Sha0StateLayout_cycleArg(b: BoundLayout__Arguments_Sha0StateLayout) -> BoundLayout_CycleArgLayout5LayoutArray {
  return BoundLayout_CycleArgLayout5LayoutArray(b.lyt.cycleArg, b.buf);
}
struct ShaEcallLayout {
  _super: ShaStateLayout,
  stateInAddr: ReadAddrLayout,
  stateOutAddr: ReadAddrLayout,
  dataAddr: ReadAddrLayout,
  _0: MemoryReadLayout,
  kAddr: ReadAddrLayout,
}
struct BoundLayout_ShaEcallLayout {
  lyt: ShaEcallLayout,
  buf: u32,
}
fn lookup_ShaEcallLayout__super(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaEcallLayout_stateInAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.stateInAddr, b.buf);
}
fn lookup_ShaEcallLayout_stateOutAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.stateOutAddr, b.buf);
}
fn lookup_ShaEcallLayout_dataAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.dataAddr, b.buf);
}
fn lookup_ShaEcallLayout__0(b: BoundLayout_ShaEcallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt._0, b.buf);
}
fn lookup_ShaEcallLayout_kAddr(b: BoundLayout_ShaEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.kAddr, b.buf);
}
struct ShaLoadStateLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  countZero: IsZeroLayout,
  a32: MemoryReadLayout,
  e32: MemoryReadLayout,
  _0: MemoryWriteLayout,
  _1: MemoryWriteLayout,
}
struct BoundLayout_ShaLoadStateLayout {
  lyt: ShaLoadStateLayout,
  buf: u32,
}
fn lookup_ShaLoadStateLayout__super(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaLoadStateLayout_lastRound(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaLoadStateLayout_countZero(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.countZero, b.buf);
}
fn lookup_ShaLoadStateLayout_a32(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.a32, b.buf);
}
fn lookup_ShaLoadStateLayout_e32(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.e32, b.buf);
}
fn lookup_ShaLoadStateLayout__0(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
fn lookup_ShaLoadStateLayout__1(b: BoundLayout_ShaLoadStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._1, b.buf);
}
struct Sha0StateArm1Layout {
  _super: ShaLoadStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm1Layout {
  lyt: Sha0StateArm1Layout,
  buf: u32,
}
fn lookup_Sha0StateArm1Layout__super(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_ShaLoadStateLayout {
  return BoundLayout_ShaLoadStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm1Layout__extra0(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm1Layout__extra1(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm1Layout__extra2(b: BoundLayout_Sha0StateArm1Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra2, b.buf);
}
struct UnpackReg_32__16_Layout {
  _super: NondetRegLayout32LayoutArray,
}
struct BoundLayout_UnpackReg_32__16_Layout {
  lyt: UnpackReg_32__16_Layout,
  buf: u32,
}
fn lookup_UnpackReg_32__16_Layout__super(b: BoundLayout_UnpackReg_32__16_Layout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt._super, b.buf);
}
struct CarryExtractLayout {
  bit0: NondetRegLayout,
  bit1: NondetRegLayout,
  bit2: NondetRegLayout,
}
struct BoundLayout_CarryExtractLayout {
  lyt: CarryExtractLayout,
  buf: u32,
}
fn lookup_CarryExtractLayout_bit0(b: BoundLayout_CarryExtractLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bit0, b.buf);
}
fn lookup_CarryExtractLayout_bit1(b: BoundLayout_CarryExtractLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bit1, b.buf);
}
fn lookup_CarryExtractLayout_bit2(b: BoundLayout_CarryExtractLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.bit2, b.buf);
}
struct CarryAndExpandLayout {
  _super: UnpackReg_32__16_Layout,
  lowCarry: CarryExtractLayout,
  highCarry: CarryExtractLayout,
}
struct BoundLayout_CarryAndExpandLayout {
  lyt: CarryAndExpandLayout,
  buf: u32,
}
fn lookup_CarryAndExpandLayout__super(b: BoundLayout_CarryAndExpandLayout) -> BoundLayout_UnpackReg_32__16_Layout {
  return BoundLayout_UnpackReg_32__16_Layout(b.lyt._super, b.buf);
}
fn lookup_CarryAndExpandLayout_lowCarry(b: BoundLayout_CarryAndExpandLayout) -> BoundLayout_CarryExtractLayout {
  return BoundLayout_CarryExtractLayout(b.lyt.lowCarry, b.buf);
}
fn lookup_CarryAndExpandLayout_highCarry(b: BoundLayout_CarryAndExpandLayout) -> BoundLayout_CarryExtractLayout {
  return BoundLayout_CarryExtractLayout(b.lyt.highCarry, b.buf);
}
struct ShaLoadDataLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  k: MemoryReadLayout,
  wMem: MemoryReadLayout,
  wBits: NondetRegLayout32LayoutArray,
  a: CarryAndExpandLayout,
  e: CarryAndExpandLayout,
}
struct BoundLayout_ShaLoadDataLayout {
  lyt: ShaLoadDataLayout,
  buf: u32,
}
fn lookup_ShaLoadDataLayout__super(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaLoadDataLayout_lastRound(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaLoadDataLayout_k(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.k, b.buf);
}
fn lookup_ShaLoadDataLayout_wMem(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.wMem, b.buf);
}
fn lookup_ShaLoadDataLayout_wBits(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_NondetRegLayout32LayoutArray {
  return BoundLayout_NondetRegLayout32LayoutArray(b.lyt.wBits, b.buf);
}
fn lookup_ShaLoadDataLayout_a(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.a, b.buf);
}
fn lookup_ShaLoadDataLayout_e(b: BoundLayout_ShaLoadDataLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.e, b.buf);
}
struct Sha0StateArm2Layout {
  _super: ShaLoadDataLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: CycleArgLayout,
  _extra7: CycleArgLayout,
  _extra8: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm2Layout {
  lyt: Sha0StateArm2Layout,
  buf: u32,
}
fn lookup_Sha0StateArm2Layout__super(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_ShaLoadDataLayout {
  return BoundLayout_ShaLoadDataLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra0(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra1(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra2(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra3(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra4(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra5(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra6(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra7(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm2Layout__extra8(b: BoundLayout_Sha0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
struct ShaMixLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  k: MemoryReadLayout,
  wBits: CarryAndExpandLayout,
  a: CarryAndExpandLayout,
  e: CarryAndExpandLayout,
}
struct BoundLayout_ShaMixLayout {
  lyt: ShaMixLayout,
  buf: u32,
}
fn lookup_ShaMixLayout__super(b: BoundLayout_ShaMixLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaMixLayout_lastRound(b: BoundLayout_ShaMixLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaMixLayout_k(b: BoundLayout_ShaMixLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.k, b.buf);
}
fn lookup_ShaMixLayout_wBits(b: BoundLayout_ShaMixLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.wBits, b.buf);
}
fn lookup_ShaMixLayout_a(b: BoundLayout_ShaMixLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.a, b.buf);
}
fn lookup_ShaMixLayout_e(b: BoundLayout_ShaMixLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.e, b.buf);
}
struct Sha0StateArm3Layout {
  _super: ShaMixLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm3Layout {
  lyt: Sha0StateArm3Layout,
  buf: u32,
}
fn lookup_Sha0StateArm3Layout__super(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_ShaMixLayout {
  return BoundLayout_ShaMixLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra0(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra1(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra2(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra3(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra4(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra5(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra6(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra7(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra8(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra9(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra10(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm3Layout__extra11(b: BoundLayout_Sha0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
struct ShaStoreStateLayout {
  _super: ShaStateLayout,
  lastRound: IsZeroLayout,
  countZero: IsZeroLayout,
  a: CarryAndExpandLayout,
  e: CarryAndExpandLayout,
  _1: MemoryWriteLayout,
  _2: MemoryWriteLayout,
}
struct BoundLayout_ShaStoreStateLayout {
  lyt: ShaStoreStateLayout,
  buf: u32,
}
fn lookup_ShaStoreStateLayout__super(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_ShaStoreStateLayout_lastRound(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.lastRound, b.buf);
}
fn lookup_ShaStoreStateLayout_countZero(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt.countZero, b.buf);
}
fn lookup_ShaStoreStateLayout_a(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.a, b.buf);
}
fn lookup_ShaStoreStateLayout_e(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_CarryAndExpandLayout {
  return BoundLayout_CarryAndExpandLayout(b.lyt.e, b.buf);
}
fn lookup_ShaStoreStateLayout__1(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._1, b.buf);
}
fn lookup_ShaStoreStateLayout__2(b: BoundLayout_ShaStoreStateLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._2, b.buf);
}
struct Sha0StateArm4Layout {
  _super: ShaStoreStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: CycleArgLayout,
  _extra7: CycleArgLayout,
  _extra8: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm4Layout {
  lyt: Sha0StateArm4Layout,
  buf: u32,
}
fn lookup_Sha0StateArm4Layout__super(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_ShaStoreStateLayout {
  return BoundLayout_ShaStoreStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra0(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra1(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra2(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra3(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra4(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra5(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra6(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra7(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm4Layout__extra8(b: BoundLayout_Sha0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
struct Sha0StateArm5Layout {
  _super: ShaStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm5Layout {
  lyt: Sha0StateArm5Layout,
  buf: u32,
}
fn lookup_Sha0StateArm5Layout__super(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra0(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra1(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra2(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra3(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra4(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra5(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra6(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra7(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra8(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra9(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra10(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra11(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra12(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra13(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Sha0StateArm5Layout__extra14(b: BoundLayout_Sha0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
struct Sha0StateArm6Layout {
  _super: ShaStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm6Layout {
  lyt: Sha0StateArm6Layout,
  buf: u32,
}
fn lookup_Sha0StateArm6Layout__super(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra0(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra1(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra2(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra3(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra4(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra5(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra6(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra7(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra8(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra9(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra10(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra11(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra12(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra13(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Sha0StateArm6Layout__extra14(b: BoundLayout_Sha0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
struct Sha0StateArm7Layout {
  _super: ShaStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
}
struct BoundLayout_Sha0StateArm7Layout {
  lyt: Sha0StateArm7Layout,
  buf: u32,
}
fn lookup_Sha0StateArm7Layout__super(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra0(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra1(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra2(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra3(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra4(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra5(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra6(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra7(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra8(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra9(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra10(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra11(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra12(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra13(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_Sha0StateArm7Layout__extra14(b: BoundLayout_Sha0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
struct Sha0StateLayout {
  _super: ShaStateLayout,
  arm0: ShaEcallLayout,
  arm1: Sha0StateArm1Layout,
  arm2: Sha0StateArm2Layout,
  arm3: Sha0StateArm3Layout,
  arm4: Sha0StateArm4Layout,
  arm5: Sha0StateArm5Layout,
  arm6: Sha0StateArm6Layout,
  arm7: Sha0StateArm7Layout,
}
struct BoundLayout_Sha0StateLayout {
  lyt: Sha0StateLayout,
  buf: u32,
}
fn lookup_Sha0StateLayout__super(b: BoundLayout_Sha0StateLayout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt._super, b.buf);
}
fn lookup_Sha0StateLayout_arm0(b: BoundLayout_Sha0StateLayout) -> BoundLayout_ShaEcallLayout {
  return BoundLayout_ShaEcallLayout(b.lyt.arm0, b.buf);
}
fn lookup_Sha0StateLayout_arm1(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm1Layout {
  return BoundLayout_Sha0StateArm1Layout(b.lyt.arm1, b.buf);
}
fn lookup_Sha0StateLayout_arm2(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm2Layout {
  return BoundLayout_Sha0StateArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_Sha0StateLayout_arm3(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm3Layout {
  return BoundLayout_Sha0StateArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_Sha0StateLayout_arm4(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm4Layout {
  return BoundLayout_Sha0StateArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_Sha0StateLayout_arm5(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm5Layout {
  return BoundLayout_Sha0StateArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_Sha0StateLayout_arm6(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm6Layout {
  return BoundLayout_Sha0StateArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_Sha0StateLayout_arm7(b: BoundLayout_Sha0StateLayout) -> BoundLayout_Sha0StateArm7Layout {
  return BoundLayout_Sha0StateArm7Layout(b.lyt.arm7, b.buf);
}
struct Sha0Layout {
  _0: DoCycleTableLayout,
  state: ShaStateLayout,
  _arguments_Sha0State: _Arguments_Sha0StateLayout,
  stateRedef: Sha0StateLayout,
}
struct BoundLayout_Sha0Layout {
  lyt: Sha0Layout,
  buf: u32,
}
fn lookup_Sha0Layout__0(b: BoundLayout_Sha0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_Sha0Layout_state(b: BoundLayout_Sha0Layout) -> BoundLayout_ShaStateLayout {
  return BoundLayout_ShaStateLayout(b.lyt.state, b.buf);
}
fn lookup_Sha0Layout__arguments_Sha0State(b: BoundLayout_Sha0Layout) -> BoundLayout__Arguments_Sha0StateLayout {
  return BoundLayout__Arguments_Sha0StateLayout(b.lyt._arguments_Sha0State, b.buf);
}
fn lookup_Sha0Layout_stateRedef(b: BoundLayout_Sha0Layout) -> BoundLayout_Sha0StateLayout {
  return BoundLayout_Sha0StateLayout(b.lyt.stateRedef, b.buf);
}
struct BigIntStateLayout {
  isEcall: NondetRegLayout,
  mode: NondetRegLayout,
  pc: NondetRegLayout,
  polyOp: NondetRegLayout,
  coeff: NondetRegLayout,
  bytes: NondetRegLayout16LayoutArray,
  nextState: NondetRegLayout,
}
struct BoundLayout_BigIntStateLayout {
  lyt: BigIntStateLayout,
  buf: u32,
}
fn lookup_BigIntStateLayout_isEcall(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isEcall, b.buf);
}
fn lookup_BigIntStateLayout_mode(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.mode, b.buf);
}
fn lookup_BigIntStateLayout_pc(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.pc, b.buf);
}
fn lookup_BigIntStateLayout_polyOp(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.polyOp, b.buf);
}
fn lookup_BigIntStateLayout_coeff(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.coeff, b.buf);
}
fn lookup_BigIntStateLayout_bytes(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout16LayoutArray {
  return BoundLayout_NondetRegLayout16LayoutArray(b.lyt.bytes, b.buf);
}
fn lookup_BigIntStateLayout_nextState(b: BoundLayout_BigIntStateLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState, b.buf);
}
alias MemoryArgLayout12LayoutArray = array<MemoryArgLayout, 12>;
struct BoundLayout_MemoryArgLayout12LayoutArray {
  lyt: MemoryArgLayout12LayoutArray,
  buf: u32,
}
fn subscript_MemoryArgLayout12LayoutArray(b: BoundLayout_MemoryArgLayout12LayoutArray, i: u32) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt[i], b.buf);
}
alias CycleArgLayout6LayoutArray = array<CycleArgLayout, 6>;
struct BoundLayout_CycleArgLayout6LayoutArray {
  lyt: CycleArgLayout6LayoutArray,
  buf: u32,
}
fn subscript_CycleArgLayout6LayoutArray(b: BoundLayout_CycleArgLayout6LayoutArray, i: u32) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt[i], b.buf);
}
alias ArgU8Layout18LayoutArray = array<ArgU8Layout, 18>;
struct BoundLayout_ArgU8Layout18LayoutArray {
  lyt: ArgU8Layout18LayoutArray,
  buf: u32,
}
fn subscript_ArgU8Layout18LayoutArray(b: BoundLayout_ArgU8Layout18LayoutArray, i: u32) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt[i], b.buf);
}
struct _Arguments_BigInt0StateLayout {
  memoryArg: MemoryArgLayout12LayoutArray,
  cycleArg: CycleArgLayout6LayoutArray,
  argU8: ArgU8Layout18LayoutArray,
  argU16: ArgU16Layout4LayoutArray,
}
struct BoundLayout__Arguments_BigInt0StateLayout {
  lyt: _Arguments_BigInt0StateLayout,
  buf: u32,
}
fn lookup__Arguments_BigInt0StateLayout_memoryArg(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_MemoryArgLayout12LayoutArray {
  return BoundLayout_MemoryArgLayout12LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_BigInt0StateLayout_cycleArg(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_CycleArgLayout6LayoutArray {
  return BoundLayout_CycleArgLayout6LayoutArray(b.lyt.cycleArg, b.buf);
}
fn lookup__Arguments_BigInt0StateLayout_argU8(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_ArgU8Layout18LayoutArray {
  return BoundLayout_ArgU8Layout18LayoutArray(b.lyt.argU8, b.buf);
}
fn lookup__Arguments_BigInt0StateLayout_argU16(b: BoundLayout__Arguments_BigInt0StateLayout) -> BoundLayout_ArgU16Layout4LayoutArray {
  return BoundLayout_ArgU16Layout4LayoutArray(b.lyt.argU16, b.buf);
}
struct BigIntEcallLayout {
  _super: BigIntStateLayout,
  mode: MemoryReadLayout,
  pc: ReadAddrLayout,
}
struct BoundLayout_BigIntEcallLayout {
  lyt: BigIntEcallLayout,
  buf: u32,
}
fn lookup_BigIntEcallLayout__super(b: BoundLayout_BigIntEcallLayout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntEcallLayout_mode(b: BoundLayout_BigIntEcallLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.mode, b.buf);
}
fn lookup_BigIntEcallLayout_pc(b: BoundLayout_BigIntEcallLayout) -> BoundLayout_ReadAddrLayout {
  return BoundLayout_ReadAddrLayout(b.lyt.pc, b.buf);
}
struct BigInt0StateArm0Layout {
  _super: BigIntEcallLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: CycleArgLayout,
  _extra9: CycleArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: ArgU8Layout,
  _extra13: ArgU8Layout,
  _extra14: ArgU8Layout,
  _extra15: ArgU8Layout,
  _extra16: ArgU8Layout,
  _extra17: ArgU8Layout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU16Layout,
  _extra31: ArgU16Layout,
  _extra32: ArgU16Layout,
  _extra33: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm0Layout {
  lyt: BigInt0StateArm0Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm0Layout__super(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_BigIntEcallLayout {
  return BoundLayout_BigIntEcallLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra0(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra1(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra2(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra3(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra4(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra5(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra6(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra7(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra8(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra9(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra10(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra11(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra12(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra13(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra14(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra15(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra16(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra17(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra18(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra19(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra20(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra21(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra22(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra23(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra24(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra25(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra26(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra27(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra28(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra29(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra30(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra31(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra32(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm0Layout__extra33(b: BoundLayout_BigInt0StateArm0Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra33, b.buf);
}
struct _Arguments_BigIntStepBytesLayout {
  argU16: ArgU16Layout2LayoutArray,
  argU8: ArgU8Layout16LayoutArray,
  memoryArg: MemoryArgLayout8LayoutArray,
  cycleArg: CycleArgLayout4LayoutArray,
}
struct BoundLayout__Arguments_BigIntStepBytesLayout {
  lyt: _Arguments_BigIntStepBytesLayout,
  buf: u32,
}
fn lookup__Arguments_BigIntStepBytesLayout_argU16(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_ArgU16Layout2LayoutArray {
  return BoundLayout_ArgU16Layout2LayoutArray(b.lyt.argU16, b.buf);
}
fn lookup__Arguments_BigIntStepBytesLayout_argU8(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_ArgU8Layout16LayoutArray {
  return BoundLayout_ArgU8Layout16LayoutArray(b.lyt.argU8, b.buf);
}
fn lookup__Arguments_BigIntStepBytesLayout_memoryArg(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_MemoryArgLayout8LayoutArray {
  return BoundLayout_MemoryArgLayout8LayoutArray(b.lyt.memoryArg, b.buf);
}
fn lookup__Arguments_BigIntStepBytesLayout_cycleArg(b: BoundLayout__Arguments_BigIntStepBytesLayout) -> BoundLayout_CycleArgLayout4LayoutArray {
  return BoundLayout_CycleArgLayout4LayoutArray(b.lyt.cycleArg, b.buf);
}
struct BigIntAddrLayout {
  _super: AddrDecomposeBitsLayout,
  _0: IsZeroLayout,
}
struct BoundLayout_BigIntAddrLayout {
  lyt: BigIntAddrLayout,
  buf: u32,
}
fn lookup_BigIntAddrLayout__super(b: BoundLayout_BigIntAddrLayout) -> BoundLayout_AddrDecomposeBitsLayout {
  return BoundLayout_AddrDecomposeBitsLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntAddrLayout__0(b: BoundLayout_BigIntAddrLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._0, b.buf);
}
struct SplitU32Layout {
  low: SplitWordLayout,
  high: SplitWordLayout,
}
struct BoundLayout_SplitU32Layout {
  lyt: SplitU32Layout,
  buf: u32,
}
fn lookup_SplitU32Layout_low(b: BoundLayout_SplitU32Layout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.low, b.buf);
}
fn lookup_SplitU32Layout_high(b: BoundLayout_SplitU32Layout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.high, b.buf);
}
struct BigIntReadWords_SuperLayout {
  _super: SplitU32Layout,
  _0: MemoryReadLayout,
}
struct BoundLayout_BigIntReadWords_SuperLayout {
  lyt: BigIntReadWords_SuperLayout,
  buf: u32,
}
fn lookup_BigIntReadWords_SuperLayout__super(b: BoundLayout_BigIntReadWords_SuperLayout) -> BoundLayout_SplitU32Layout {
  return BoundLayout_SplitU32Layout(b.lyt._super, b.buf);
}
fn lookup_BigIntReadWords_SuperLayout__0(b: BoundLayout_BigIntReadWords_SuperLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt._0, b.buf);
}
alias BigIntReadWords_SuperLayout4LayoutArray = array<BigIntReadWords_SuperLayout, 4>;
struct BoundLayout_BigIntReadWords_SuperLayout4LayoutArray {
  lyt: BigIntReadWords_SuperLayout4LayoutArray,
  buf: u32,
}
fn subscript_BigIntReadWords_SuperLayout4LayoutArray(b: BoundLayout_BigIntReadWords_SuperLayout4LayoutArray, i: u32) -> BoundLayout_BigIntReadWords_SuperLayout {
  return BoundLayout_BigIntReadWords_SuperLayout(b.lyt[i], b.buf);
}
struct BigIntReadLayout {
  addr: BigIntAddrLayout,
  words: BigIntReadWords_SuperLayout4LayoutArray,
}
struct BoundLayout_BigIntReadLayout {
  lyt: BigIntReadLayout,
  buf: u32,
}
fn lookup_BigIntReadLayout_addr(b: BoundLayout_BigIntReadLayout) -> BoundLayout_BigIntAddrLayout {
  return BoundLayout_BigIntAddrLayout(b.lyt.addr, b.buf);
}
fn lookup_BigIntReadLayout_words(b: BoundLayout_BigIntReadLayout) -> BoundLayout_BigIntReadWords_SuperLayout4LayoutArray {
  return BoundLayout_BigIntReadWords_SuperLayout4LayoutArray(b.lyt.words, b.buf);
}
alias NondetU8RegLayout16LayoutArray = array<NondetU8RegLayout, 16>;
struct BoundLayout_NondetU8RegLayout16LayoutArray {
  lyt: NondetU8RegLayout16LayoutArray,
  buf: u32,
}
fn subscript_NondetU8RegLayout16LayoutArray(b: BoundLayout_NondetU8RegLayout16LayoutArray, i: u32) -> BoundLayout_NondetU8RegLayout {
  return BoundLayout_NondetU8RegLayout(b.lyt[i], b.buf);
}
struct BigIntWitnessLayout {
  _super: NondetU8RegLayout16LayoutArray,
}
struct BoundLayout_BigIntWitnessLayout {
  lyt: BigIntWitnessLayout,
  buf: u32,
}
fn lookup_BigIntWitnessLayout__super(b: BoundLayout_BigIntWitnessLayout) -> BoundLayout_NondetU8RegLayout16LayoutArray {
  return BoundLayout_NondetU8RegLayout16LayoutArray(b.lyt._super, b.buf);
}
struct BigIntWrite__0_SuperLayout {
  _0: MemoryWriteLayout,
}
struct BoundLayout_BigIntWrite__0_SuperLayout {
  lyt: BigIntWrite__0_SuperLayout,
  buf: u32,
}
fn lookup_BigIntWrite__0_SuperLayout__0(b: BoundLayout_BigIntWrite__0_SuperLayout) -> BoundLayout_MemoryWriteLayout {
  return BoundLayout_MemoryWriteLayout(b.lyt._0, b.buf);
}
alias BigIntWrite__0_SuperLayout4LayoutArray = array<BigIntWrite__0_SuperLayout, 4>;
struct BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray {
  lyt: BigIntWrite__0_SuperLayout4LayoutArray,
  buf: u32,
}
fn subscript_BigIntWrite__0_SuperLayout4LayoutArray(b: BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray, i: u32) -> BoundLayout_BigIntWrite__0_SuperLayout {
  return BoundLayout_BigIntWrite__0_SuperLayout(b.lyt[i], b.buf);
}
struct BigIntWriteLayout {
  _super: BigIntWitnessLayout,
  addr: BigIntAddrLayout,
  _1: BigIntWrite__0_SuperLayout4LayoutArray,
}
struct BoundLayout_BigIntWriteLayout {
  lyt: BigIntWriteLayout,
  buf: u32,
}
fn lookup_BigIntWriteLayout__super(b: BoundLayout_BigIntWriteLayout) -> BoundLayout_BigIntWitnessLayout {
  return BoundLayout_BigIntWitnessLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntWriteLayout_addr(b: BoundLayout_BigIntWriteLayout) -> BoundLayout_BigIntAddrLayout {
  return BoundLayout_BigIntAddrLayout(b.lyt.addr, b.buf);
}
fn lookup_BigIntWriteLayout__1(b: BoundLayout_BigIntWriteLayout) -> BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray {
  return BoundLayout_BigIntWrite__0_SuperLayout4LayoutArray(b.lyt._1, b.buf);
}
struct BigIntStepBytesArm2Layout {
  _super: BigIntWitnessLayout,
  _extra0: ArgU16Layout,
  _extra1: ArgU16Layout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: CycleArgLayout,
  _extra11: CycleArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
}
struct BoundLayout_BigIntStepBytesArm2Layout {
  lyt: BigIntStepBytesArm2Layout,
  buf: u32,
}
fn lookup_BigIntStepBytesArm2Layout__super(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_BigIntWitnessLayout {
  return BoundLayout_BigIntWitnessLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra0(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra0, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra1(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra1, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra2(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra3(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra4(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra5(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra6(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra7(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra8(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra9(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra10(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra11(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra12(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigIntStepBytesArm2Layout__extra13(b: BoundLayout_BigIntStepBytesArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
struct BigIntStepBytesLayout {
  arm0: BigIntReadLayout,
  arm1: BigIntWriteLayout,
  arm2: BigIntStepBytesArm2Layout,
}
struct BoundLayout_BigIntStepBytesLayout {
  lyt: BigIntStepBytesLayout,
  buf: u32,
}
fn lookup_BigIntStepBytesLayout_arm0(b: BoundLayout_BigIntStepBytesLayout) -> BoundLayout_BigIntReadLayout {
  return BoundLayout_BigIntReadLayout(b.lyt.arm0, b.buf);
}
fn lookup_BigIntStepBytesLayout_arm1(b: BoundLayout_BigIntStepBytesLayout) -> BoundLayout_BigIntWriteLayout {
  return BoundLayout_BigIntWriteLayout(b.lyt.arm1, b.buf);
}
fn lookup_BigIntStepBytesLayout_arm2(b: BoundLayout_BigIntStepBytesLayout) -> BoundLayout_BigIntStepBytesArm2Layout {
  return BoundLayout_BigIntStepBytesArm2Layout(b.lyt.arm2, b.buf);
}
struct BigIntStepLayout {
  _super: BigIntStateLayout,
  loadInst_0: MemoryReadLayout,
  instHigh: SplitWordLayout,
  polyOp: NondetRegLayout,
  memOp: NondetRegLayout,
  regBits: NondetRegLayout5LayoutArray,
  coeffBits: NondetRegLayout3LayoutArray,
  baseAddrU32: MemoryReadLayout,
  dataAddrU32: NormalizeU32Layout,
  memOpOneHot: OneHot_3_Layout,
  _arguments_BigIntStepBytes: _Arguments_BigIntStepBytesLayout,
  bytes: BigIntStepBytesLayout,
  _2: IsZeroLayout,
}
struct BoundLayout_BigIntStepLayout {
  lyt: BigIntStepLayout,
  buf: u32,
}
fn lookup_BigIntStepLayout__super(b: BoundLayout_BigIntStepLayout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigIntStepLayout_loadInst_0(b: BoundLayout_BigIntStepLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.loadInst_0, b.buf);
}
fn lookup_BigIntStepLayout_instHigh(b: BoundLayout_BigIntStepLayout) -> BoundLayout_SplitWordLayout {
  return BoundLayout_SplitWordLayout(b.lyt.instHigh, b.buf);
}
fn lookup_BigIntStepLayout_polyOp(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.polyOp, b.buf);
}
fn lookup_BigIntStepLayout_memOp(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.memOp, b.buf);
}
fn lookup_BigIntStepLayout_regBits(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout5LayoutArray {
  return BoundLayout_NondetRegLayout5LayoutArray(b.lyt.regBits, b.buf);
}
fn lookup_BigIntStepLayout_coeffBits(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NondetRegLayout3LayoutArray {
  return BoundLayout_NondetRegLayout3LayoutArray(b.lyt.coeffBits, b.buf);
}
fn lookup_BigIntStepLayout_baseAddrU32(b: BoundLayout_BigIntStepLayout) -> BoundLayout_MemoryReadLayout {
  return BoundLayout_MemoryReadLayout(b.lyt.baseAddrU32, b.buf);
}
fn lookup_BigIntStepLayout_dataAddrU32(b: BoundLayout_BigIntStepLayout) -> BoundLayout_NormalizeU32Layout {
  return BoundLayout_NormalizeU32Layout(b.lyt.dataAddrU32, b.buf);
}
fn lookup_BigIntStepLayout_memOpOneHot(b: BoundLayout_BigIntStepLayout) -> BoundLayout_OneHot_3_Layout {
  return BoundLayout_OneHot_3_Layout(b.lyt.memOpOneHot, b.buf);
}
fn lookup_BigIntStepLayout__arguments_BigIntStepBytes(b: BoundLayout_BigIntStepLayout) -> BoundLayout__Arguments_BigIntStepBytesLayout {
  return BoundLayout__Arguments_BigIntStepBytesLayout(b.lyt._arguments_BigIntStepBytes, b.buf);
}
fn lookup_BigIntStepLayout_bytes(b: BoundLayout_BigIntStepLayout) -> BoundLayout_BigIntStepBytesLayout {
  return BoundLayout_BigIntStepBytesLayout(b.lyt.bytes, b.buf);
}
fn lookup_BigIntStepLayout__2(b: BoundLayout_BigIntStepLayout) -> BoundLayout_IsZeroLayout {
  return BoundLayout_IsZeroLayout(b.lyt._2, b.buf);
}
struct BigInt0StateArm2Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm2Layout {
  lyt: BigInt0StateArm2Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm2Layout__super(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra0(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra1(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra2(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra3(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra4(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra5(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra6(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra7(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra8(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra9(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra10(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra11(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra12(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra13(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra14(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra15(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra16(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra17(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra18(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra19(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra20(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra21(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra22(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra23(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra24(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra25(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra26(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra27(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra28(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra29(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra30(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra31(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra32(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra33(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra34(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra35(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra36(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra37(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra38(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm2Layout__extra39(b: BoundLayout_BigInt0StateArm2Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm3Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm3Layout {
  lyt: BigInt0StateArm3Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm3Layout__super(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra0(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra1(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra2(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra3(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra4(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra5(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra6(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra7(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra8(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra9(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra10(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra11(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra12(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra13(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra14(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra15(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra16(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra17(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra18(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra19(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra20(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra21(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra22(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra23(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra24(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra25(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra26(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra27(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra28(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra29(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra30(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra31(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra32(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra33(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra34(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra35(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra36(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra37(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra38(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm3Layout__extra39(b: BoundLayout_BigInt0StateArm3Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm4Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm4Layout {
  lyt: BigInt0StateArm4Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm4Layout__super(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra0(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra1(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra2(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra3(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra4(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra5(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra6(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra7(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra8(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra9(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra10(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra11(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra12(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra13(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra14(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra15(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra16(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra17(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra18(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra19(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra20(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra21(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra22(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra23(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra24(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra25(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra26(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra27(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra28(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra29(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra30(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra31(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra32(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra33(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra34(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra35(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra36(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra37(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra38(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm4Layout__extra39(b: BoundLayout_BigInt0StateArm4Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm5Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm5Layout {
  lyt: BigInt0StateArm5Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm5Layout__super(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra0(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra1(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra2(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra3(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra4(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra5(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra6(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra7(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra8(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra9(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra10(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra11(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra12(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra13(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra14(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra15(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra16(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra17(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra18(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra19(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra20(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra21(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra22(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra23(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra24(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra25(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra26(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra27(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra28(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra29(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra30(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra31(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra32(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra33(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra34(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra35(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra36(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra37(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra38(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm5Layout__extra39(b: BoundLayout_BigInt0StateArm5Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm6Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm6Layout {
  lyt: BigInt0StateArm6Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm6Layout__super(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra0(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra1(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra2(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra3(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra4(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra5(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra6(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra7(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra8(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra9(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra10(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra11(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra12(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra13(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra14(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra15(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra16(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra17(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra18(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra19(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra20(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra21(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra22(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra23(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra24(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra25(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra26(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra27(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra28(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra29(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra30(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra31(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra32(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra33(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra34(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra35(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra36(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra37(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra38(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm6Layout__extra39(b: BoundLayout_BigInt0StateArm6Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateArm7Layout {
  _super: BigIntStateLayout,
  _extra0: MemoryArgLayout,
  _extra1: MemoryArgLayout,
  _extra2: MemoryArgLayout,
  _extra3: MemoryArgLayout,
  _extra4: MemoryArgLayout,
  _extra5: MemoryArgLayout,
  _extra6: MemoryArgLayout,
  _extra7: MemoryArgLayout,
  _extra8: MemoryArgLayout,
  _extra9: MemoryArgLayout,
  _extra10: MemoryArgLayout,
  _extra11: MemoryArgLayout,
  _extra12: CycleArgLayout,
  _extra13: CycleArgLayout,
  _extra14: CycleArgLayout,
  _extra15: CycleArgLayout,
  _extra16: CycleArgLayout,
  _extra17: CycleArgLayout,
  _extra18: ArgU8Layout,
  _extra19: ArgU8Layout,
  _extra20: ArgU8Layout,
  _extra21: ArgU8Layout,
  _extra22: ArgU8Layout,
  _extra23: ArgU8Layout,
  _extra24: ArgU8Layout,
  _extra25: ArgU8Layout,
  _extra26: ArgU8Layout,
  _extra27: ArgU8Layout,
  _extra28: ArgU8Layout,
  _extra29: ArgU8Layout,
  _extra30: ArgU8Layout,
  _extra31: ArgU8Layout,
  _extra32: ArgU8Layout,
  _extra33: ArgU8Layout,
  _extra34: ArgU8Layout,
  _extra35: ArgU8Layout,
  _extra36: ArgU16Layout,
  _extra37: ArgU16Layout,
  _extra38: ArgU16Layout,
  _extra39: ArgU16Layout,
}
struct BoundLayout_BigInt0StateArm7Layout {
  lyt: BigInt0StateArm7Layout,
  buf: u32,
}
fn lookup_BigInt0StateArm7Layout__super(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra0(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra0, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra1(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra1, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra2(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra2, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra3(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra3, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra4(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra4, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra5(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra5, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra6(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra6, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra7(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra7, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra8(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra8, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra9(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra9, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra10(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra10, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra11(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_MemoryArgLayout {
  return BoundLayout_MemoryArgLayout(b.lyt._extra11, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra12(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra12, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra13(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra13, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra14(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra14, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra15(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra15, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra16(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra16, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra17(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_CycleArgLayout {
  return BoundLayout_CycleArgLayout(b.lyt._extra17, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra18(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra18, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra19(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra19, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra20(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra20, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra21(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra21, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra22(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra22, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra23(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra23, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra24(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra24, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra25(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra25, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra26(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra26, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra27(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra27, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra28(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra28, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra29(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra29, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra30(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra30, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra31(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra31, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra32(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra32, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra33(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra33, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra34(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra34, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra35(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU8Layout {
  return BoundLayout_ArgU8Layout(b.lyt._extra35, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra36(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra36, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra37(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra37, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra38(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra38, b.buf);
}
fn lookup_BigInt0StateArm7Layout__extra39(b: BoundLayout_BigInt0StateArm7Layout) -> BoundLayout_ArgU16Layout {
  return BoundLayout_ArgU16Layout(b.lyt._extra39, b.buf);
}
struct BigInt0StateLayout {
  _super: BigIntStateLayout,
  arm0: BigInt0StateArm0Layout,
  arm1: BigIntStepLayout,
  arm2: BigInt0StateArm2Layout,
  arm3: BigInt0StateArm3Layout,
  arm4: BigInt0StateArm4Layout,
  arm5: BigInt0StateArm5Layout,
  arm6: BigInt0StateArm6Layout,
  arm7: BigInt0StateArm7Layout,
}
struct BoundLayout_BigInt0StateLayout {
  lyt: BigInt0StateLayout,
  buf: u32,
}
fn lookup_BigInt0StateLayout__super(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt._super, b.buf);
}
fn lookup_BigInt0StateLayout_arm0(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm0Layout {
  return BoundLayout_BigInt0StateArm0Layout(b.lyt.arm0, b.buf);
}
fn lookup_BigInt0StateLayout_arm1(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigIntStepLayout {
  return BoundLayout_BigIntStepLayout(b.lyt.arm1, b.buf);
}
fn lookup_BigInt0StateLayout_arm2(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm2Layout {
  return BoundLayout_BigInt0StateArm2Layout(b.lyt.arm2, b.buf);
}
fn lookup_BigInt0StateLayout_arm3(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm3Layout {
  return BoundLayout_BigInt0StateArm3Layout(b.lyt.arm3, b.buf);
}
fn lookup_BigInt0StateLayout_arm4(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm4Layout {
  return BoundLayout_BigInt0StateArm4Layout(b.lyt.arm4, b.buf);
}
fn lookup_BigInt0StateLayout_arm5(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm5Layout {
  return BoundLayout_BigInt0StateArm5Layout(b.lyt.arm5, b.buf);
}
fn lookup_BigInt0StateLayout_arm6(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm6Layout {
  return BoundLayout_BigInt0StateArm6Layout(b.lyt.arm6, b.buf);
}
fn lookup_BigInt0StateLayout_arm7(b: BoundLayout_BigInt0StateLayout) -> BoundLayout_BigInt0StateArm7Layout {
  return BoundLayout_BigInt0StateArm7Layout(b.lyt.arm7, b.buf);
}
struct BigInt0Layout {
  _0: DoCycleTableLayout,
  state: BigIntStateLayout,
  _arguments_BigInt0State: _Arguments_BigInt0StateLayout,
  stateRedef: BigInt0StateLayout,
}
struct BoundLayout_BigInt0Layout {
  lyt: BigInt0Layout,
  buf: u32,
}
fn lookup_BigInt0Layout__0(b: BoundLayout_BigInt0Layout) -> BoundLayout_DoCycleTableLayout {
  return BoundLayout_DoCycleTableLayout(b.lyt._0, b.buf);
}
fn lookup_BigInt0Layout_state(b: BoundLayout_BigInt0Layout) -> BoundLayout_BigIntStateLayout {
  return BoundLayout_BigIntStateLayout(b.lyt.state, b.buf);
}
fn lookup_BigInt0Layout__arguments_BigInt0State(b: BoundLayout_BigInt0Layout) -> BoundLayout__Arguments_BigInt0StateLayout {
  return BoundLayout__Arguments_BigInt0StateLayout(b.lyt._arguments_BigInt0State, b.buf);
}
fn lookup_BigInt0Layout_stateRedef(b: BoundLayout_BigInt0Layout) -> BoundLayout_BigInt0StateLayout {
  return BoundLayout_BigInt0StateLayout(b.lyt.stateRedef, b.buf);
}
struct TopInstResultLayout {
  _selector: NondetRegLayout13LayoutArray,
  arm0: Misc0Layout,
  arm1: Misc1Layout,
  arm2: Misc2Layout,
  arm3: Mul0Layout,
  arm4: Div0Layout,
  arm5: Mem0Layout,
  arm6: Mem1Layout,
  arm7: Control0Layout,
  arm8: ECall0Layout,
  arm9: Poseidon0Layout,
  arm10: Poseidon1Layout,
  arm11: Sha0Layout,
  arm12: BigInt0Layout,
}
struct BoundLayout_TopInstResultLayout {
  lyt: TopInstResultLayout,
  buf: u32,
}
fn lookup_TopInstResultLayout__selector(b: BoundLayout_TopInstResultLayout) -> BoundLayout_NondetRegLayout13LayoutArray {
  return BoundLayout_NondetRegLayout13LayoutArray(b.lyt._selector, b.buf);
}
fn lookup_TopInstResultLayout_arm0(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Misc0Layout {
  return BoundLayout_Misc0Layout(b.lyt.arm0, b.buf);
}
fn lookup_TopInstResultLayout_arm1(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Misc1Layout {
  return BoundLayout_Misc1Layout(b.lyt.arm1, b.buf);
}
fn lookup_TopInstResultLayout_arm2(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Misc2Layout {
  return BoundLayout_Misc2Layout(b.lyt.arm2, b.buf);
}
fn lookup_TopInstResultLayout_arm3(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Mul0Layout {
  return BoundLayout_Mul0Layout(b.lyt.arm3, b.buf);
}
fn lookup_TopInstResultLayout_arm4(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Div0Layout {
  return BoundLayout_Div0Layout(b.lyt.arm4, b.buf);
}
fn lookup_TopInstResultLayout_arm5(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Mem0Layout {
  return BoundLayout_Mem0Layout(b.lyt.arm5, b.buf);
}
fn lookup_TopInstResultLayout_arm6(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Mem1Layout {
  return BoundLayout_Mem1Layout(b.lyt.arm6, b.buf);
}
fn lookup_TopInstResultLayout_arm7(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Control0Layout {
  return BoundLayout_Control0Layout(b.lyt.arm7, b.buf);
}
fn lookup_TopInstResultLayout_arm8(b: BoundLayout_TopInstResultLayout) -> BoundLayout_ECall0Layout {
  return BoundLayout_ECall0Layout(b.lyt.arm8, b.buf);
}
fn lookup_TopInstResultLayout_arm9(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Poseidon0Layout {
  return BoundLayout_Poseidon0Layout(b.lyt.arm9, b.buf);
}
fn lookup_TopInstResultLayout_arm10(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Poseidon1Layout {
  return BoundLayout_Poseidon1Layout(b.lyt.arm10, b.buf);
}
fn lookup_TopInstResultLayout_arm11(b: BoundLayout_TopInstResultLayout) -> BoundLayout_Sha0Layout {
  return BoundLayout_Sha0Layout(b.lyt.arm11, b.buf);
}
fn lookup_TopInstResultLayout_arm12(b: BoundLayout_TopInstResultLayout) -> BoundLayout_BigInt0Layout {
  return BoundLayout_BigInt0Layout(b.lyt.arm12, b.buf);
}
struct TopCycleLayout {
  _super: NondetRegLayout,
  arm0: NondetRegLayout,
  arm1: NondetRegLayout,
}
struct BoundLayout_TopCycleLayout {
  lyt: TopCycleLayout,
  buf: u32,
}
fn lookup_TopCycleLayout__super(b: BoundLayout_TopCycleLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt._super, b.buf);
}
fn lookup_TopCycleLayout_arm0(b: BoundLayout_TopCycleLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.arm0, b.buf);
}
fn lookup_TopCycleLayout_arm1(b: BoundLayout_TopCycleLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.arm1, b.buf);
}
struct TopLayout {
  cycle: NondetRegLayout,
  nextPcLow: NondetRegLayout,
  nextPcHigh: NondetRegLayout,
  nextState_0: NondetRegLayout,
  nextMachineMode: NondetRegLayout,
  isFirstCycle: NondetRegLayout,
  cycleRedef: TopCycleLayout,
  major: NondetRegLayout,
  minor: NondetRegLayout,
  instInput: InstInputLayout,
  majorOnehot: OneHot_13_Layout,
  instResult: TopInstResultLayout,
}
struct BoundLayout_TopLayout {
  lyt: TopLayout,
  buf: u32,
}
fn lookup_TopLayout_cycle(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.cycle, b.buf);
}
fn lookup_TopLayout_nextPcLow(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextPcLow, b.buf);
}
fn lookup_TopLayout_nextPcHigh(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextPcHigh, b.buf);
}
fn lookup_TopLayout_nextState_0(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextState_0, b.buf);
}
fn lookup_TopLayout_nextMachineMode(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.nextMachineMode, b.buf);
}
fn lookup_TopLayout_isFirstCycle(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isFirstCycle, b.buf);
}
fn lookup_TopLayout_cycleRedef(b: BoundLayout_TopLayout) -> BoundLayout_TopCycleLayout {
  return BoundLayout_TopCycleLayout(b.lyt.cycleRedef, b.buf);
}
fn lookup_TopLayout_major(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.major, b.buf);
}
fn lookup_TopLayout_minor(b: BoundLayout_TopLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.minor, b.buf);
}
fn lookup_TopLayout_instInput(b: BoundLayout_TopLayout) -> BoundLayout_InstInputLayout {
  return BoundLayout_InstInputLayout(b.lyt.instInput, b.buf);
}
fn lookup_TopLayout_majorOnehot(b: BoundLayout_TopLayout) -> BoundLayout_OneHot_13_Layout {
  return BoundLayout_OneHot_13_Layout(b.lyt.majorOnehot, b.buf);
}
fn lookup_TopLayout_instResult(b: BoundLayout_TopLayout) -> BoundLayout_TopInstResultLayout {
  return BoundLayout_TopInstResultLayout(b.lyt.instResult, b.buf);
}
struct DigestRegValues_SuperLayout {
  low: NondetRegLayout,
  high: NondetRegLayout,
}
struct BoundLayout_DigestRegValues_SuperLayout {
  lyt: DigestRegValues_SuperLayout,
  buf: u32,
}
fn lookup_DigestRegValues_SuperLayout_low(b: BoundLayout_DigestRegValues_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.low, b.buf);
}
fn lookup_DigestRegValues_SuperLayout_high(b: BoundLayout_DigestRegValues_SuperLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.high, b.buf);
}
alias DigestRegValues_SuperLayout8LayoutArray = array<DigestRegValues_SuperLayout, 8>;
struct BoundLayout_DigestRegValues_SuperLayout8LayoutArray {
  lyt: DigestRegValues_SuperLayout8LayoutArray,
  buf: u32,
}
fn subscript_DigestRegValues_SuperLayout8LayoutArray(b: BoundLayout_DigestRegValues_SuperLayout8LayoutArray, i: u32) -> BoundLayout_DigestRegValues_SuperLayout {
  return BoundLayout_DigestRegValues_SuperLayout(b.lyt[i], b.buf);
}
struct DigestRegLayout {
  values: DigestRegValues_SuperLayout8LayoutArray,
}
struct BoundLayout_DigestRegLayout {
  lyt: DigestRegLayout,
  buf: u32,
}
fn lookup_DigestRegLayout_values(b: BoundLayout_DigestRegLayout) -> BoundLayout_DigestRegValues_SuperLayout8LayoutArray {
  return BoundLayout_DigestRegValues_SuperLayout8LayoutArray(b.lyt.values, b.buf);
}
struct Arg_ArgU8Layout {
  val: Reg,
}
struct BoundLayout_Arg_ArgU8Layout {
  lyt: Arg_ArgU8Layout,
  buf: u32,
}
fn lookup_Arg_ArgU8Layout_val(b: BoundLayout_Arg_ArgU8Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.val, b.buf);
}
struct Arg_ArgU16Layout {
  val: Reg,
}
struct BoundLayout_Arg_ArgU16Layout {
  lyt: Arg_ArgU16Layout,
  buf: u32,
}
fn lookup_Arg_ArgU16Layout_val(b: BoundLayout_Arg_ArgU16Layout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.val, b.buf);
}
struct Arg_MemoryArgLayout {
  addr: Reg,
  cycle: Reg,
  dataLow: Reg,
  dataHigh: Reg,
}
struct BoundLayout_Arg_MemoryArgLayout {
  lyt: Arg_MemoryArgLayout,
  buf: u32,
}
fn lookup_Arg_MemoryArgLayout_addr(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.addr, b.buf);
}
fn lookup_Arg_MemoryArgLayout_cycle(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.cycle, b.buf);
}
fn lookup_Arg_MemoryArgLayout_dataLow(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.dataLow, b.buf);
}
fn lookup_Arg_MemoryArgLayout_dataHigh(b: BoundLayout_Arg_MemoryArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.dataHigh, b.buf);
}
struct Arg_CycleArgLayout {
  cycle: Reg,
}
struct BoundLayout_Arg_CycleArgLayout {
  lyt: Arg_CycleArgLayout,
  buf: u32,
}
fn lookup_Arg_CycleArgLayout_cycle(b: BoundLayout_Arg_CycleArgLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt.cycle, b.buf);
}
alias Reg1LayoutArray = array<Reg, 1>;
struct BoundLayout_Reg1LayoutArray {
  lyt: Reg1LayoutArray,
  buf: u32,
}
fn subscript_Reg1LayoutArray(b: BoundLayout_Reg1LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct _accumLayout {
  argU8: Arg_ArgU8Layout,
  argU16: Arg_ArgU16Layout,
  memoryArg: Arg_MemoryArgLayout,
  cycleArg: Arg_CycleArgLayout,
  _offset: Reg,
  _user: Reg1LayoutArray,
}
struct BoundLayout__accumLayout {
  lyt: _accumLayout,
  buf: u32,
}
fn lookup__accumLayout_argU8(b: BoundLayout__accumLayout) -> BoundLayout_Arg_ArgU8Layout {
  return BoundLayout_Arg_ArgU8Layout(b.lyt.argU8, b.buf);
}
fn lookup__accumLayout_argU16(b: BoundLayout__accumLayout) -> BoundLayout_Arg_ArgU16Layout {
  return BoundLayout_Arg_ArgU16Layout(b.lyt.argU16, b.buf);
}
fn lookup__accumLayout_memoryArg(b: BoundLayout__accumLayout) -> BoundLayout_Arg_MemoryArgLayout {
  return BoundLayout_Arg_MemoryArgLayout(b.lyt.memoryArg, b.buf);
}
fn lookup__accumLayout_cycleArg(b: BoundLayout__accumLayout) -> BoundLayout_Arg_CycleArgLayout {
  return BoundLayout_Arg_CycleArgLayout(b.lyt.cycleArg, b.buf);
}
fn lookup__accumLayout__offset(b: BoundLayout__accumLayout) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt._offset, b.buf);
}
fn lookup__accumLayout__user(b: BoundLayout__accumLayout) -> BoundLayout_Reg1LayoutArray {
  return BoundLayout_Reg1LayoutArray(b.lyt._user, b.buf);
}
alias Reg20LayoutArray = array<Reg, 20>;
struct BoundLayout_Reg20LayoutArray {
  lyt: Reg20LayoutArray,
  buf: u32,
}
fn subscript_Reg20LayoutArray(b: BoundLayout_Reg20LayoutArray, i: u32) -> BoundLayout_Reg {
  return BoundLayout_Reg(b.lyt[i], b.buf);
}
struct LayoutAccumLayout {
  user: AccumLayout,
  columns: Reg20LayoutArray,
}
struct BoundLayout_LayoutAccumLayout {
  lyt: LayoutAccumLayout,
  buf: u32,
}
fn lookup_LayoutAccumLayout_user(b: BoundLayout_LayoutAccumLayout) -> BoundLayout_AccumLayout {
  return BoundLayout_AccumLayout(b.lyt.user, b.buf);
}
fn lookup_LayoutAccumLayout_columns(b: BoundLayout_LayoutAccumLayout) -> BoundLayout_Reg20LayoutArray {
  return BoundLayout_Reg20LayoutArray(b.lyt.columns, b.buf);
}
struct TestSuccRunLayout {
  _0: TopLayout,
}
struct BoundLayout_TestSuccRunLayout {
  lyt: TestSuccRunLayout,
  buf: u32,
}
fn lookup_TestSuccRunLayout__0(b: BoundLayout_TestSuccRunLayout) -> BoundLayout_TopLayout {
  return BoundLayout_TopLayout(b.lyt._0, b.buf);
}
struct _globalLayout {
  input: DigestRegLayout,
  isTerminate: NondetRegLayout,
  output: DigestRegLayout,
  povwNonce: DigestRegLayout,
  rng: NondetExtRegLayout,
  shutdownCycle: NondetRegLayout,
  stateIn: DigestRegLayout,
  stateOut: DigestRegLayout,
  termA0high: NondetRegLayout,
  termA0low: NondetRegLayout,
  termA1high: NondetRegLayout,
  termA1low: NondetRegLayout,
}
struct BoundLayout__globalLayout {
  lyt: _globalLayout,
  buf: u32,
}
fn lookup__globalLayout_input(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.input, b.buf);
}
fn lookup__globalLayout_isTerminate(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.isTerminate, b.buf);
}
fn lookup__globalLayout_output(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.output, b.buf);
}
fn lookup__globalLayout_povwNonce(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.povwNonce, b.buf);
}
fn lookup__globalLayout_rng(b: BoundLayout__globalLayout) -> BoundLayout_NondetExtRegLayout {
  return BoundLayout_NondetExtRegLayout(b.lyt.rng, b.buf);
}
fn lookup__globalLayout_shutdownCycle(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.shutdownCycle, b.buf);
}
fn lookup__globalLayout_stateIn(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.stateIn, b.buf);
}
fn lookup__globalLayout_stateOut(b: BoundLayout__globalLayout) -> BoundLayout_DigestRegLayout {
  return BoundLayout_DigestRegLayout(b.lyt.stateOut, b.buf);
}
fn lookup__globalLayout_termA0high(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA0high, b.buf);
}
fn lookup__globalLayout_termA0low(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA0low, b.buf);
}
fn lookup__globalLayout_termA1high(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA1high, b.buf);
}
fn lookup__globalLayout_termA1low(b: BoundLayout__globalLayout) -> BoundLayout_NondetRegLayout {
  return BoundLayout_NondetRegLayout(b.lyt.termA1low, b.buf);
}
struct _mixLayout {
  randomness: _accumLayout,
}
struct BoundLayout__mixLayout {
  lyt: _mixLayout,
  buf: u32,
}
fn lookup__mixLayout_randomness(b: BoundLayout__mixLayout) -> BoundLayout__accumLayout {
  return BoundLayout__accumLayout(b.lyt.randomness, b.buf);
}
struct NondetRegStruct {
  _super: Val,
}
struct NondetExtRegStruct {
  _super: ExtVal,
}
struct NondetFakeTwitRegStruct {
  _super: Val,
}
struct FakeTwitRegStruct {
  _unused: u32,
}
struct ArgU8Struct {
  count: NondetRegStruct,
  val: NondetRegStruct,
}
struct U8RegStruct {
  _unused: u32,
}
struct ArgU16Struct {
  count: NondetRegStruct,
  val: NondetRegStruct,
}
struct NondetU16RegStruct {
  _super: NondetRegStruct,
}
alias Val5Array = array<Val, 5>;
alias Val30Array = array<Val, 30>;
alias NondetRegStruct5Array = array<NondetRegStruct, 5>;
struct ToBits_5_Struct {
  _super: NondetRegStruct5Array,
}
struct ValU32Struct {
  low: Val,
  high: Val,
}
struct DenormedValU32Struct {
  low: Val,
  high: Val,
}
struct NormalizeU32Struct {
  _super: ValU32Struct,
  highCarry: NondetRegStruct,
}
struct AddrDecomposeStruct {
  _super: Val,
  low2: NondetRegStruct,
}
struct AddrDecomposeBitsStruct {
  _super: Val,
  low0: NondetRegStruct,
  low1: NondetRegStruct,
  low2: Val,
}
struct CmpEqualStruct {
  isEqual: NondetRegStruct,
}
struct CmpLessThanUnsignedStruct {
  isLessThan: Val,
}
struct CmpLessThanStruct {
  isLessThan: NondetRegStruct,
}
alias Val16Array = array<Val, 16>;
alias NondetRegStruct16Array = array<NondetRegStruct, 16>;
struct ToBits_16_Struct {
  _super: NondetRegStruct16Array,
}
struct FromBits_16_Struct {
  _super: Val,
}
struct DecoderStruct {
  opcode: NondetRegStruct,
  rs1: Val,
  rs2: Val,
  rd: Val,
  func7: Val,
  func3: Val,
  immI: ValU32Struct,
  immS: ValU32Struct,
  immB: ValU32Struct,
  immU: ValU32Struct,
  immJ: ValU32Struct,
}
struct MemoryArgStruct {
  count: NondetRegStruct,
  addr: NondetRegStruct,
  cycle: NondetRegStruct,
  dataLow: NondetRegStruct,
  dataHigh: NondetRegStruct,
}
struct CycleArgStruct {
  count: NondetRegStruct,
  cycle: NondetRegStruct,
}
struct IsCycleStruct {
  _unused: u32,
}
struct MemoryIOStruct {
  oldTxn: MemoryArgStruct,
  newTxn: MemoryArgStruct,
}
struct IsForwardStruct {
  _unused: u32,
}
struct GetDataStruct {
  _super: ValU32Struct,
  diffLow: Val,
  diffHigh: Val,
}
struct MemoryWriteStruct {
  _unused: u32,
}
struct MemoryWriteUnconstrainedStruct {
  io: MemoryIOStruct,
}
alias Val3Array = array<Val, 3>;
alias NondetRegStruct3Array = array<NondetRegStruct, 3>;
struct OneHot_3_Struct {
  _super: NondetRegStruct3Array,
}
alias Val8Array = array<Val, 8>;
alias NondetRegStruct8Array = array<NondetRegStruct, 8>;
struct OneHot_8_Struct {
  _super: NondetRegStruct8Array,
}
struct InstInputStruct {
  minor: Val,
  pcU32: ValU32Struct,
  state: Val,
  mode: Val,
  minorOnehot: OneHot_8_Struct,
}
struct DoCycleTableStruct {
  _unused: u32,
}
struct SourceRegsStruct {
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct ReadSourceRegsStruct {
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct WriteRdStruct {
  _unused: u32,
}
struct ExpandU32Struct {
  b0: NondetRegStruct,
  b1: NondetRegStruct,
  b2: NondetRegStruct,
  b3: NondetRegStruct,
  neg: Val,
}
struct SplitTotalStruct {
  out_: NondetU16RegStruct,
  carry: Val,
}
struct MultiplySettingsStruct {
  aSigned: Val,
  bSigned: Val,
  cSigned: Val,
}
struct MultiplyAccumulateStruct {
  outLow: ValU32Struct,
  outHigh: ValU32Struct,
  bNeg: Val,
}
struct DivInputStruct {
  _super: InstInputStruct,
  decoded: DecoderStruct,
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct ComponentStruct {
  _unused: u32,
}
struct DivideReturnStruct {
  quot: ValU32Struct,
  rem: ValU32Struct,
}
struct BigIntTopStateStruct {
  polyOp: Val,
  coeff: Val,
  witness: Val16Array,
}
struct InstOutputBaseStruct {
  newPc: ValU32Struct,
  newState: Val,
  newMode: Val,
  topState: BigIntTopStateStruct,
}
struct MiscInputStruct {
  _super: InstInputStruct,
  decoded: DecoderStruct,
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct MiscOutputStruct {
  doWrite: Val,
  toWrite: DenormedValU32Struct,
  newPc: DenormedValU32Struct,
}
struct MulInputStruct {
  _super: InstInputStruct,
  decoded: DecoderStruct,
  rs1: ValU32Struct,
  rs2: ValU32Struct,
}
struct DoMulStruct {
  low: ValU32Struct,
  high: ValU32Struct,
}
struct MemLoadInputStruct {
  ii: InstInputStruct,
  decoded: DecoderStruct,
  addr: AddrDecomposeBitsStruct,
  data: GetDataStruct,
}
struct MemStoreInputStruct {
  decoded: DecoderStruct,
  rs2: ValU32Struct,
  addr: AddrDecomposeBitsStruct,
  data: GetDataStruct,
}
struct MemStoreFinalizeStruct {
  _unused: u32,
}
struct SplitWordStruct {
  byte0: NondetRegStruct,
  byte1: NondetRegStruct,
}
struct DigestRegValues_SuperStruct {
  low: NondetRegStruct,
  high: NondetRegStruct,
}
alias DigestRegValues_SuperStruct8Array = array<DigestRegValues_SuperStruct, 8>;
struct DigestRegStruct {
  values: DigestRegValues_SuperStruct8Array,
}
alias ValU32Struct8Array = array<ValU32Struct, 8>;
alias GetDataStruct8Array = array<GetDataStruct, 8>;
struct ControlResume_SuperArm1_Super__0Struct {
  _unused: u32,
}
alias ControlResume_SuperArm1_Super__0Struct8Array = array<ControlResume_SuperArm1_Super__0Struct, 8>;
struct ControlTable_SuperArm0_Super__0Struct {
  _unused: u32,
}
struct ControlTable_SuperArm1_Super__0Struct {
  _unused: u32,
}
alias ControlTable_SuperArm0_Super__0Struct16Array = array<ControlTable_SuperArm0_Super__0Struct, 16>;
alias ControlTable_SuperArm1_Super__0Struct16Array = array<ControlTable_SuperArm1_Super__0Struct, 16>;
alias Val6Array = array<Val, 6>;
alias NondetRegStruct6Array = array<NondetRegStruct, 6>;
struct OneHot_6_Struct {
  _super: NondetRegStruct6Array,
}
struct ECallOutputStruct {
  state: Val,
  s0: Val,
  s1: Val,
  s2: Val,
}
alias Val4Array = array<Val, 4>;
alias NondetRegStruct4Array = array<NondetRegStruct, 4>;
struct OneHot_4_Struct {
  _super: NondetRegStruct4Array,
}
struct DecomposeLow2Struct {
  high: NondetU16RegStruct,
  low2: NondetRegStruct,
  low2Hot: OneHot_4_Struct,
  highZero: NondetRegStruct,
  isZero: NondetRegStruct,
  low2Nonzero: Val,
}
struct ECallHostReadWords__0Struct {
  _unused: u32,
}
alias ECallHostReadWords__0Struct4Array = array<ECallHostReadWords__0Struct, 4>;
alias Val24Array = array<Val, 24>;
struct MultiplyByMInt_Super_SuperStruct {
  _super: Val,
}
alias MultiplyByMInt_Super_SuperStruct24Array = array<MultiplyByMInt_Super_SuperStruct, 24>;
struct MultiplyByMIntStruct {
  _super: MultiplyByMInt_Super_SuperStruct24Array,
}
struct DoIntRounds__0_SuperStruct {
  _super: Val,
}
alias DoIntRounds__0_SuperStruct21Array = array<DoIntRounds__0_SuperStruct, 21>;
struct DoIntRoundsStruct {
  _super: Val24Array,
}
alias NondetRegStruct24Array = array<NondetRegStruct, 24>;
struct MultiplyByMExt_Super_SuperStruct {
  _super: Val,
}
alias MultiplyByMExt_Super_SuperStruct24Array = array<MultiplyByMExt_Super_SuperStruct, 24>;
struct MultiplyByMExtStruct {
  _super: MultiplyByMExt_Super_SuperStruct24Array,
}
struct PoseidonStateStruct {
  hasState: NondetRegStruct,
  stateAddr: NondetRegStruct,
  bufOutAddr: NondetRegStruct,
  isElem: NondetRegStruct,
  checkOut: NondetRegStruct,
  loadTxType: NondetRegStruct,
  nextState: NondetRegStruct,
  subState: NondetRegStruct,
  bufInAddr: NondetRegStruct,
  count: NondetRegStruct,
  mode: NondetRegStruct,
  inner: NondetRegStruct24Array,
  zcheck: NondetExtRegStruct,
}
struct PoseidonOpDefStruct {
  hasState: Val,
  stateAddr: Val,
  bufOutAddr: Val,
  isElem: Val,
  checkOut: Val,
  loadTxType: Val,
}
struct ReadAddrStruct {
  _super: Val,
}
struct ReadElemStruct {
  _super: Val,
}
alias ReadElemStruct8Array = array<ReadElemStruct, 8>;
struct PoseidonCheckOut__0Struct {
  _unused: u32,
}
alias PoseidonCheckOut__0Struct8Array = array<PoseidonCheckOut__0Struct, 8>;
struct FieldToWordStruct {
  ret: ValU32Struct,
}
struct PoseidonStoreOut__0Struct {
  _unused: u32,
}
alias PoseidonStoreOut__0Struct8Array = array<PoseidonStoreOut__0Struct, 8>;
struct PoseidonStoreState__0Struct {
  _unused: u32,
}
alias PoseidonStoreState__0Struct8Array = array<PoseidonStoreState__0Struct, 8>;
struct IsU24Struct {
  _unused: u32,
}
struct CarryExtractStruct {
  carry: Val,
  out_: Val,
}
alias Val2Array = array<Val, 2>;
struct DivStruct {
  _super: Val,
}
alias DivStruct32Array = array<DivStruct, 32>;
alias Val32Array = array<Val, 32>;
alias NondetRegStruct32Array = array<NondetRegStruct, 32>;
struct UnpackReg_32__16_Struct {
  _super: NondetRegStruct32Array,
}
struct ShaStateAStruct {
  _super: NondetRegStruct,
}
alias ShaStateAStruct32Array = array<ShaStateAStruct, 32>;
struct ShaStateEStruct {
  _super: NondetRegStruct,
}
alias ShaStateEStruct32Array = array<ShaStateEStruct, 32>;
struct ShaStateWStruct {
  _super: NondetRegStruct,
}
alias ShaStateWStruct32Array = array<ShaStateWStruct, 32>;
struct ShaStateStruct {
  stateInAddr: NondetRegStruct,
  stateOutAddr: NondetRegStruct,
  dataAddr: NondetRegStruct,
  count: NondetRegStruct,
  kAddr: NondetRegStruct,
  round: NondetRegStruct,
  nextState: NondetRegStruct,
  a: ShaStateAStruct32Array,
  e: ShaStateEStruct32Array,
  w: ShaStateWStruct32Array,
}
struct BigIntStateStruct {
  isEcall: NondetRegStruct,
  mode: NondetRegStruct,
  pc: NondetRegStruct,
  polyOp: NondetRegStruct,
  coeff: NondetRegStruct,
  bytes: NondetRegStruct16Array,
  nextState: NondetRegStruct,
}
struct SplitU32Struct {
  bytes: NondetRegStruct4Array,
}
alias SplitU32Struct4Array = array<SplitU32Struct, 4>;
struct BigIntReadStruct {
  _super: NondetRegStruct16Array,
}
struct BigIntWitnessStruct {
  _super: NondetRegStruct16Array,
}
struct BigIntWrite__0Struct {
  _unused: u32,
}
alias BigIntWrite__0Struct4Array = array<BigIntWrite__0Struct, 4>;
struct BigIntAccumStateStruct {
  poly: NondetExtRegStruct,
  term: NondetExtRegStruct,
  total: NondetExtRegStruct,
}
alias Val7Array = array<Val, 7>;
alias NondetRegStruct7Array = array<NondetRegStruct, 7>;
struct OneHot_7_Struct {
  _super: NondetRegStruct7Array,
}
alias ExtVal1Array = array<ExtVal, 1>;
struct BigIntAccumStruct {
  _unused: u32,
}
alias Val13Array = array<Val, 13>;
alias NondetRegStruct13Array = array<NondetRegStruct, 13>;
struct OneHot_13_Struct {
  _super: NondetRegStruct13Array,
}
struct TopStruct {
  _unused: u32,
}
struct AccumStruct {
  _unused: u32,
}
