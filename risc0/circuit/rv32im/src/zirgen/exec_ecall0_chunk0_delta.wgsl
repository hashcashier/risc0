fn exec_NondetReg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
store(lookup_NondetRegLayout__super(layout1), arg0);
let x2: NondetRegStruct = NondetRegStruct(load(lookup_NondetRegLayout__super(layout1), 0));
return x2;
}
fn exec_Reg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
let x2: NondetRegStruct = exec_NondetReg(arg0, layout1);
// Reg(<preamble>:6)
eqz(sub(arg0, x2._super));
return x2;
}
fn exec_NondetBitReg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
let x2: NondetRegStruct = exec_NondetReg(arg0, layout1);
// builtin Mul
// AssertBit(zirgen/circuit/rv32im/v2/dsl/bits.zir:7)
// NondetBitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:14)
let x3: Val = mul(x2._super, sub(268435454u, x2._super));
eqz(x3);
return x2;
}
fn exec_IsZero(arg0: Val, layout1: BoundLayout_IsZeroLayout) -> NondetRegStruct {
// IsZero(zirgen/circuit/rv32im/v2/dsl/is_zero.zir:8)
let x2: NondetRegStruct = exec_NondetReg(isz(arg0), lookup_IsZeroLayout__super(layout1));
// IsZero(zirgen/circuit/rv32im/v2/dsl/is_zero.zir:11)
let x3: NondetRegStruct = exec_NondetReg(inv_0(arg0), lookup_IsZeroLayout_inv(layout1));
// builtin Sub
// AssertBit(zirgen/circuit/rv32im/v2/dsl/bits.zir:7)
// IsZero(zirgen/circuit/rv32im/v2/dsl/is_zero.zir:14)
let x4: Val = sub(268435454u, x2._super);
eqz(mul(x2._super, x4));
// IsZero(zirgen/circuit/rv32im/v2/dsl/is_zero.zir:16)
eqz(sub(mul(arg0, x3._super), x4));
// IsZero(zirgen/circuit/rv32im/v2/dsl/is_zero.zir:18)
eqz(mul(x2._super, arg0));
// IsZero(zirgen/circuit/rv32im/v2/dsl/is_zero.zir:20)
eqz(mul(x2._super, x3._super));
return x2;
}
fn exec_ArgU16(arg0: Val, arg1_0: Val, layout2: BoundLayout_ArgU16Layout) -> ArgU16Struct {
// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:33)
let x3: NondetRegStruct = exec_NondetReg(arg0, lookup_ArgU16Layout_count(layout2));
// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:34)
let x4: NondetRegStruct = exec_NondetReg(arg1_0, lookup_ArgU16Layout_val(layout2));
// LookupDelta(zirgen/circuit/rv32im/v2/dsl/lookups.zir:4)
// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:35)
extern_lookupDelta(268435422u, x4._super, x3._super);
// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:36)
let x5: Val = sub(268435454u, inRange(0u, x4._super, 268295646u));
extern_noop();
return ArgU16Struct(x3, x4);
}
fn exec_NondetU16Reg(arg0: Val, layout1: BoundLayout_NondetU16RegLayout) -> NondetU16RegStruct {
// NondetU16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:42)
let x2: ArgU16Struct = exec_ArgU16(268435454u, arg0, lookup_NondetU16RegLayout_arg(layout1));
// NondetU16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:43)
let x3: Val = sub(x2.count._super, 268435454u);
eqz(x3);
return NondetU16RegStruct(x2.val);
}
fn exec_U16Reg(arg0: Val, layout1: BoundLayout_NondetU16RegLayout) -> NondetU16RegStruct {
// U16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:50)
let x2: NondetU16RegStruct = exec_NondetU16Reg(arg0, layout1);
// U16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:51)
let x3: Val = sub(x2._super._super, arg0);
eqz(x3);
return x2;
}
fn exec_NormalizeU32(arg0: DenormedValU32Struct, layout1: BoundLayout_NormalizeU32Layout) -> NormalizeU32Struct {
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:44)
let x2: NondetU16RegStruct = exec_NondetU16Reg(bitAnd(arg0.low, 2013126113u), lookup_NormalizeU32Layout_low16(layout1));
// builtin Mul
// Div(<preamble>:19)
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:45)
let x3: Val = mul(bitAnd(arg0.low, 268295646u), 65536u);
let x4: NondetRegStruct = exec_NondetBitReg(x3, lookup_NormalizeU32Layout_lowCarry(layout1));
// builtin Add
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:46)
let x5: Val = add(mul(x4._super, 268295646u), x2._super._super);
eqz(sub(arg0.low, x5));
// builtin Add
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:48)
let x6: Val = add(arg0.high, x4._super);
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:50)
let x7: NondetU16RegStruct = exec_NondetU16Reg(bitAnd(x6, 2013126113u), lookup_NormalizeU32Layout_high16(layout1));
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:51)
let x8: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(x6, 268295646u), 65536u), lookup_NormalizeU32Layout_highCarry(layout1));
// builtin Add
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:52)
let x9: Val = add(mul(x8._super, 268295646u), x7._super._super);
eqz(sub(x6, x9));
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:56)
let x10: ValU32Struct = ValU32Struct(x2._super._super, x7._super._super);
return NormalizeU32Struct(x10, x8);
}
fn exec_AddrDecomposeBits(arg0: ValU32Struct, arg1_0: Val, layout2: BoundLayout_AddrDecomposeBitsLayout) -> AddrDecomposeBitsStruct {
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:81)
let x3: NondetRegStruct = exec_NondetBitReg(bitAnd(arg0.low, 268435454u), lookup_AddrDecomposeBitsLayout_low0(layout2));
// builtin Mul
// Div(<preamble>:19)
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:82)
let x4: Val = mul(bitAnd(arg0.low, 536870908u), 134217727u);
let x5: NondetRegStruct = exec_NondetBitReg(x4, lookup_AddrDecomposeBitsLayout_low1(layout2));
// builtin Add
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:83)
let x6: Val = add(mul(x5._super, 536870908u), x3._super);
// builtin Mul
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:85)
let x7: Val = mul(sub(268435454u, arg1_0), 939419241u);
// builtin Sub
let x8: Val = sub(add(mul(arg1_0, 2013126113u), x7), arg0.high);
let x9: NondetU16RegStruct = exec_U16Reg(x8, lookup_AddrDecomposeBitsLayout_upperDiff(layout2));
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:87)
let x10: NondetRegStruct = exec_IsZero(arg0.high, lookup_AddrDecomposeBitsLayout__0(layout2));
eqz(x10._super);
// builtin Mul
// Div(<preamble>:19)
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:89)
let x11: Val = mul(sub(arg0.low, x6), 1073741824u);
let x12: NondetU16RegStruct = exec_NondetU16Reg(x11, lookup_AddrDecomposeBitsLayout_med14(layout2));
// builtin Mul
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:91)
let x13: Val = mul(x12._super._super, 1073741816u);
eqz(sub(add(x13, x6), arg0.low));
// builtin Add
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:93)
let x14: Val = add(mul(arg0.high, 1073706872u), x12._super._super);
return AddrDecomposeBitsStruct(x14, x3, x5, x6);
}
fn exec_MemoryArg(arg0: Val, arg1_0: Val, arg2_0: Val, arg3: ValU32Struct, layout4: BoundLayout_MemoryArgLayout) -> MemoryArgStruct {
// MemoryArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:25)
let x5: NondetRegStruct = exec_NondetReg(arg0, lookup_MemoryArgLayout_count(layout4));
// MemoryArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:26)
let x6: NondetRegStruct = exec_NondetReg(arg1_0, lookup_MemoryArgLayout_addr(layout4));
// MemoryArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:27)
let x7: NondetRegStruct = exec_NondetReg(arg2_0, lookup_MemoryArgLayout_cycle(layout4));
// MemoryArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:28)
let x8: NondetRegStruct = exec_NondetReg(arg3.low, lookup_MemoryArgLayout_dataLow(layout4));
// MemoryArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:29)
let x9: NondetRegStruct = exec_NondetReg(arg3.high, lookup_MemoryArgLayout_dataHigh(layout4));
// MemoryDelta(zirgen/circuit/rv32im/v2/dsl/mem.zir:21)
// MemoryArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:30)
extern_memoryDelta(x6._super, x7._super, x8._super, x9._super, x5._super);
return MemoryArgStruct(x5, x6, x7, x8, x9);
}
fn exec_CycleArg(arg0: Val, arg1_0: Val, layout2: BoundLayout_CycleArgLayout) -> CycleArgStruct {
// CycleArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:54)
let x3: NondetRegStruct = exec_NondetReg(arg0, lookup_CycleArgLayout_count(layout2));
// CycleArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:55)
let x4: NondetRegStruct = exec_NondetReg(arg1_0, lookup_CycleArgLayout_cycle(layout2));
// LookupDelta(zirgen/circuit/rv32im/v2/dsl/lookups.zir:4)
// CycleArg(zirgen/circuit/rv32im/v2/dsl/mem.zir:56)
extern_lookupDelta(0u, x4._super, x3._super);
return CycleArgStruct(x3, x4);
}
fn exec_IsCycle(arg0: Val, layout1: BoundLayout_IsCycleLayout) -> IsCycleStruct {
// IsCycle(zirgen/circuit/rv32im/v2/dsl/mem.zir:60)
let x2: CycleArgStruct = exec_CycleArg(268435454u, arg0, lookup_IsCycleLayout_arg(layout1));
// IsCycle(zirgen/circuit/rv32im/v2/dsl/mem.zir:61)
let x3: Val = sub(x2.count._super, 268435454u);
eqz(x3);
// IsCycle(zirgen/circuit/rv32im/v2/dsl/mem.zir:62)
let x4: Val = sub(x2.cycle._super, arg0);
eqz(x4);
return IsCycleStruct(0u);
}
fn exec_MemoryIO(arg0: Val, arg1_0: Val, layout2: BoundLayout_MemoryIOLayout) -> MemoryIOStruct {
// GetMemoryTxn(zirgen/circuit/rv32im/v2/dsl/mem.zir:51)
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:66)
let x3_tuple = extern_getMemoryTxn(arg1_0);
let x3: Val = x3_tuple[0u];
let x4: Val = x3_tuple[1u];
let x5: Val = x3_tuple[2u];
let x6: Val = x3_tuple[3u];
let x7: Val = x3_tuple[4u];
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:67)
let x8: MemoryArgStruct = exec_MemoryArg(1744830467u, arg1_0, x3, ValU32Struct(x4, x5), lookup_MemoryIOLayout_oldTxn(layout2));
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:68)
let x9: MemoryArgStruct = exec_MemoryArg(268435454u, arg1_0, arg0, ValU32Struct(x6, x7), lookup_MemoryIOLayout_newTxn(layout2));
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:69)
let x10: Val = sub(x8.count._super, 1744830467u);
eqz(x10);
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:70)
let x11: Val = sub(x9.count._super, 268435454u);
eqz(x11);
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:71)
let x12: Val = sub(x9.cycle._super, arg0);
eqz(x12);
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:73)
let x13: Val = sub(x8.addr._super, x9.addr._super);
eqz(x13);
// MemoryIO(zirgen/circuit/rv32im/v2/dsl/mem.zir:74)
let x14: Val = sub(x9.addr._super, arg1_0);
eqz(x14);
return MemoryIOStruct(x8, x9);
}
fn exec_IsForward(arg0: MemoryIOStruct, layout1: BoundLayout_IsForwardLayout) -> IsForwardStruct {
// builtin Sub
// IsForward(zirgen/circuit/rv32im/v2/dsl/mem.zir:84)
let x2: Val = sub(arg0.newTxn.cycle._super, 268435454u);
let x3: IsCycleStruct = exec_IsCycle(sub(x2, arg0.oldTxn.cycle._super), lookup_IsForwardLayout__0(layout1));
return IsForwardStruct(0u);
}
fn exec_MemoryRead(arg0: NondetRegStruct, arg1_0: Val, layout2: BoundLayout_MemoryReadLayout) -> GetDataStruct {
// MemoryRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:89)
let x3: MemoryIOStruct = exec_MemoryIO(mul(arg0._super, 536870908u), arg1_0, lookup_MemoryReadLayout_io(layout2));
// IsRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:79)
// MemoryRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:90)
let x4: Val = sub(x3.oldTxn.dataLow._super, x3.newTxn.dataLow._super);
eqz(x4);
// IsRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:80)
let x5: Val = sub(x3.oldTxn.dataHigh._super, x3.newTxn.dataHigh._super);
eqz(x5);
// MemoryRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:91)
let x6: IsForwardStruct = exec_IsForward(x3, lookup_MemoryReadLayout__0(layout2));
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// GetData(zirgen/circuit/rv32im/v2/dsl/mem.zir:36)
// MemoryRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:92)
let x7: ValU32Struct = ValU32Struct(x3.newTxn.dataLow._super, x3.newTxn.dataHigh._super);
return GetDataStruct(x7, 0u, 268435454u);
}
fn exec_DoCycleTable(arg0: NondetRegStruct, layout1: BoundLayout_DoCycleTableLayout) -> DoCycleTableStruct {
// builtin Mul
// DoCycleTable(zirgen/circuit/rv32im/v2/dsl/inst.zir:19)
let x2: Val = mul(arg0._super, 536870908u);
// GetDiffCount(zirgen/circuit/rv32im/v2/dsl/mem.zir:22)
let x3: Val = extern_getDiffCount(x2);
let x4: CycleArgStruct = exec_CycleArg(neg_0(x3), x2, lookup_DoCycleTableLayout_arg1(layout1));
// builtin Add
// DoCycleTable(zirgen/circuit/rv32im/v2/dsl/inst.zir:20)
let x5: Val = add(x2, 268435454u);
// GetDiffCount(zirgen/circuit/rv32im/v2/dsl/mem.zir:22)
let x6: Val = extern_getDiffCount(x5);
let x7: CycleArgStruct = exec_CycleArg(neg_0(x6), x5, lookup_DoCycleTableLayout_arg2(layout1));
// DoCycleTable(zirgen/circuit/rv32im/v2/dsl/inst.zir:21)
let x8: Val = sub(x4.cycle._super, x2);
eqz(x8);
// DoCycleTable(zirgen/circuit/rv32im/v2/dsl/inst.zir:22)
let x9: Val = sub(x7.cycle._super, x5);
eqz(x9);
return DoCycleTableStruct(0u);
}
fn exec_OneHot_6_(arg0: Val, layout1: BoundLayout_OneHot_6_Layout) -> OneHot_6_Struct {
// builtin Isz
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:7)
let x2: Val = isz(sub(0u, arg0));
let x3: NondetRegStruct = exec_NondetBitReg(x2, subscript_NondetRegLayout6LayoutArray(lookup_OneHot_6_Layout__super(layout1), decode(0u)));
// builtin Isz
let x4: Val = isz(sub(268435454u, arg0));
let x5: NondetRegStruct = exec_NondetBitReg(x4, subscript_NondetRegLayout6LayoutArray(lookup_OneHot_6_Layout__super(layout1), decode(268435454u)));
// builtin Isz
let x6: Val = isz(sub(536870908u, arg0));
let x7: NondetRegStruct = exec_NondetBitReg(x6, subscript_NondetRegLayout6LayoutArray(lookup_OneHot_6_Layout__super(layout1), decode(536870908u)));
// builtin Isz
let x8: Val = isz(sub(805306362u, arg0));
let x9: NondetRegStruct = exec_NondetBitReg(x8, subscript_NondetRegLayout6LayoutArray(lookup_OneHot_6_Layout__super(layout1), decode(805306362u)));
// builtin Isz
let x10: Val = isz(sub(1073741816u, arg0));
let x11: NondetRegStruct = exec_NondetBitReg(x10, subscript_NondetRegLayout6LayoutArray(lookup_OneHot_6_Layout__super(layout1), decode(1073741816u)));
// builtin Isz
let x12: Val = isz(sub(1342177270u, arg0));
let x13: NondetRegStruct = exec_NondetBitReg(x12, subscript_NondetRegLayout6LayoutArray(lookup_OneHot_6_Layout__super(layout1), decode(1342177270u)));
// builtin Add
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:9)
let x14: Val = add(add(x3._super, x5._super), x7._super);
let x15: Val = add(add(add(x14, x9._super), x11._super), x13._super);
eqz(sub(x15, 268435454u));
// builtin Add
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:11)
let x16: Val = add(x5._super, mul(x7._super, 536870908u));
let x17: Val = add(add(x16, mul(x9._super, 805306362u)), mul(x11._super, 1073741816u));
eqz(sub(add(x17, mul(x13._super, 1342177270u)), arg0));
return OneHot_6_Struct(NondetRegStruct6Array(x3, x5, x7, x9, x11, x13));
}
fn exec_MachineECallChunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, arg2_0: Val, layout3: BoundLayout_MachineECallLayout) -> ECallOutputStruct {
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:26)
let x4: GetDataStruct = exec_MemoryRead(arg0, arg2_0, lookup_MachineECallLayout_loadInst(layout3));
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:27)
eqz(sub(arg1_0.state, 805306266u));
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:28)
eqz(x4._super.high);
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:29)
let x5: Val = sub(x4._super.low, 671088395u);
eqz(x5);
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:30)
eqz(sub(arg1_0.mode, 268435454u));
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:31)
let x6: GetDataStruct = exec_MemoryRead(arg0, 259522525u, lookup_MachineECallLayout_dispatchIdx(layout3));
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:32)
eqz(x6._super.high);
// MachineECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:33)
let x7: OneHot_6_Struct = exec_OneHot_6_(x6._super.low, lookup_MachineECallLayout_dispatch(layout3));
var x8: Val;
if ((x7._super[decode(0u)]._super) != 0u) {
x8 = 402653165u;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x9: ECallOutputStruct;
if ((x7._super[decode(0u)]._super) != 0u) {
x9 = ECallOutputStruct(x8, 0u, 0u, 0u);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x9;
}
fn exec_ECall0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_ECall0Layout, global3: u32) -> InstOutputBaseStruct {
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:201)
let x4: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_ECall0Layout__0(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:202)
let x5: AddrDecomposeBitsStruct = exec_AddrDecomposeBits(arg1_0.pcU32, arg1_0.mode, lookup_ECall0Layout_pcAddr(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:203)
eqz(x5.low2);
var x6: ECallOutputStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:205)
let x7: ECallOutputStruct = exec_MachineECallChunk0(arg0, arg1_0, x5._super, lookup_ECall0OutputArm0Layout__super(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:204)
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra0(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra0(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra1(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra1(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra2(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra2(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra3(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ECall0OutputArm0Layout__extra3(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_ECall0OutputArm0Layout__extra4(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_ECall0OutputArm0Layout__extra4(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_ECall0OutputArm0Layout__extra5(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_ECall0OutputArm0Layout__extra5(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra6(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra6(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra7(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra7(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra8(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra8(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra9(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_ECall0OutputArm0Layout__extra9(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra10(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra10(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra11(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra11(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra12(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra12(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra13(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_ECall0OutputArm0Layout__extra13(lookup_ECall0OutputLayout_arm0(lookup_ECall0Layout_output(layout2))))), 0));
x6 = x7;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x8: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:214)
let x9: NondetRegStruct = exec_Reg(x6.s0, lookup_ECall0Layout_s0(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:215)
let x10: NondetRegStruct = exec_Reg(x6.s1, lookup_ECall0Layout_s1(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:216)
let x11: NondetRegStruct = exec_Reg(x6.s2, lookup_ECall0Layout_s2(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:217)
let x12: NondetRegStruct = exec_IsZero(sub(x6.state, 1073741816u), lookup_ECall0Layout_isSuspend(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:218)
let x13: NondetRegStruct = exec_IsZero(sub(x6.state, 805306266u), lookup_ECall0Layout_isDecode(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:219)
let x14: NondetRegStruct = exec_IsZero(sub(x6.state, 268435422u), lookup_ECall0Layout_isP2Entry(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:220)
let x15: NondetRegStruct = exec_IsZero(sub(x6.state, 536870844u), lookup_ECall0Layout_isShaEcall(layout2));
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:221)
let x16: NondetRegStruct = exec_IsZero(sub(x6.state, 671088555u), lookup_ECall0Layout_isBigIntEcall(layout2));
// builtin Add
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:222)
let x17: Val = add(add(x12._super, x13._super), x14._super);
// builtin Mul
let x18: Val = mul(add(add(x17, x15._super), x16._super), 1073741816u);
// builtin Add
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
let x19: Val = add(arg1_0.pcU32.low, x18);
let x20: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x19, arg1_0.pcU32.high), lookup_ECall0Layout_addPC(layout2));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:223)
let x21: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
x8 = InstOutputBaseStruct(x20._super, x6.state, 268435454u, x21);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x8;
}
