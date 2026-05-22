fn back_NondetReg(distance0: Index, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
// builtin NondetReg
let x2: NondetRegStruct = NondetRegStruct(load(lookup_NondetRegLayout__super(layout1), distance0));
return x2;
}
fn exec_NondetReg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
store(lookup_NondetRegLayout__super(layout1), arg0);
let x2: NondetRegStruct = NondetRegStruct(load(lookup_NondetRegLayout__super(layout1), 0));
return x2;
}
fn back_Reg(distance0: Index, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
// Reg(<preamble>:5)
let x2: NondetRegStruct = back_NondetReg(distance0, layout1);
return x2;
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
fn exec_MemoryPageIn(arg0: NondetRegStruct, arg1_0: Val, layout2: BoundLayout_MemoryPageInLayout) -> GetDataStruct {
// MemoryPageIn(zirgen/circuit/rv32im/v2/dsl/mem.zir:112)
let x3: MemoryIOStruct = exec_MemoryIO(mul(arg0._super, 536870908u), arg1_0, lookup_MemoryPageInLayout_io(layout2));
// IsRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:79)
// MemoryPageIn(zirgen/circuit/rv32im/v2/dsl/mem.zir:113)
let x4: Val = sub(x3.oldTxn.dataLow._super, x3.newTxn.dataLow._super);
eqz(x4);
// IsRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:80)
let x5: Val = sub(x3.oldTxn.dataHigh._super, x3.newTxn.dataHigh._super);
eqz(x5);
// builtin Sub
// MemoryPageIn(zirgen/circuit/rv32im/v2/dsl/mem.zir:114)
let x6: Val = sub(x3.newTxn.cycle._super, x3.oldTxn.cycle._super);
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// GetData(zirgen/circuit/rv32im/v2/dsl/mem.zir:36)
let x7: ValU32Struct = ValU32Struct(x3.newTxn.dataLow._super, x3.newTxn.dataHigh._super);
return GetDataStruct(x7, 0u, x6);
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
fn back_DigestReg(distance0: Index, layout1: BoundLayout_DigestRegLayout) -> DigestRegStruct {
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x2: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(0u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x3: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(0u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x4: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(268435454u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x5: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(268435454u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x6: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(536870908u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x7: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(536870908u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x8: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(805306362u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x9: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(805306362u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x10: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1073741816u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x11: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1073741816u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x12: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1342177270u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x13: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1342177270u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x14: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1610612724u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x15: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1610612724u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:8)
let x16: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_low(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1879048178u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:9)
let x17: NondetRegStruct = back_Reg(distance0, lookup_DigestRegValues_SuperLayout_high(subscript_DigestRegValues_SuperLayout8LayoutArray(lookup_DigestRegLayout_values(layout1), decode(1879048178u))));
// DigestReg(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:6)
let x18: DigestRegStruct = DigestRegStruct(DigestRegValues_SuperStruct8Array(DigestRegValues_SuperStruct(x2, x3), DigestRegValues_SuperStruct(x4, x5), DigestRegValues_SuperStruct(x6, x7), DigestRegValues_SuperStruct(x8, x9), DigestRegValues_SuperStruct(x10, x11), DigestRegValues_SuperStruct(x12, x13), DigestRegValues_SuperStruct(x14, x15), DigestRegValues_SuperStruct(x16, x17)));
return x18;
}
fn exec_ControlLoadRootAndNonceChunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_ControlLoadRootAndNonceLayout, global3: u32) -> InstOutputBaseStruct {
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:18)
let x4: BoundLayout__globalLayout = BoundLayout__globalLayout(kLayoutGlobal, global3);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:23)
eqz(arg1_0.state);
// builtin Sub
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:24)
let x5: Val = sub(268435454u, arg0._super);
eqz(mul(arg0._super, x5));
// builtin Add
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:27)
let x6: Val = add(mul(arg0._super, 1592717058u), mul(x5, 1726934769u));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:29)
let x7: GetDataStruct = exec_MemoryPageIn(arg0, x6, subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(0u)));
let x8: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 268435454u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(268435454u)));
let x9: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 536870908u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(536870908u)));
let x10: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 805306362u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(805306362u)));
let x11: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 1073741816u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(1073741816u)));
let x12: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 1342177270u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(1342177270u)));
let x13: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 1610612724u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(1610612724u)));
let x14: GetDataStruct = exec_MemoryPageIn(arg0, add(x6, 1879048178u), subscript_MemoryPageInLayout8LayoutArray(lookup_ControlLoadRootAndNonceLayout_mem(layout2), decode(1879048178u)));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:40)
let x15: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
// InstOutputBase(zirgen/circuit/rv32im/v2/dsl/inst.zir:78)
let x16: InstOutputBaseStruct = InstOutputBaseStruct(ValU32Struct(0u, 0u), 0u, 0u, x15);
var x17: InstOutputBaseStruct;
if ((x5) != 0u) {
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x18: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x19: Val = sub(x7._super.low, x18.values[decode(0u)].low._super);
eqz(x19);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x20: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x21: Val = sub(x7._super.high, x20.values[decode(0u)].high._super);
eqz(x21);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x22: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x23: Val = sub(x8._super.low, x22.values[decode(268435454u)].low._super);
eqz(x23);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x24: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x25: Val = sub(x8._super.high, x24.values[decode(268435454u)].high._super);
eqz(x25);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x26: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x27: Val = sub(x9._super.low, x26.values[decode(536870908u)].low._super);
eqz(x27);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x28: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x29: Val = sub(x9._super.high, x28.values[decode(536870908u)].high._super);
eqz(x29);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x30: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x31: Val = sub(x10._super.low, x30.values[decode(805306362u)].low._super);
eqz(x31);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x32: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x33: Val = sub(x10._super.high, x32.values[decode(805306362u)].high._super);
eqz(x33);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x34: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x35: Val = sub(x11._super.low, x34.values[decode(1073741816u)].low._super);
eqz(x35);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x36: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x37: Val = sub(x11._super.high, x36.values[decode(1073741816u)].high._super);
eqz(x37);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x38: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x39: Val = sub(x12._super.low, x38.values[decode(1342177270u)].low._super);
eqz(x39);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x40: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x41: Val = sub(x12._super.high, x40.values[decode(1342177270u)].high._super);
eqz(x41);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x42: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x43: Val = sub(x13._super.low, x42.values[decode(1610612724u)].low._super);
eqz(x43);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x44: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x45: Val = sub(x13._super.high, x44.values[decode(1610612724u)].high._super);
eqz(x45);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x46: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:35)
let x47: Val = sub(x14._super.low, x46.values[decode(1879048178u)].low._super);
eqz(x47);
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:20)
let x48: DigestRegStruct = back_DigestReg(0, lookup__globalLayout_povwNonce(x4));
// ControlLoadRootAndNonce(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:36)
let x49: Val = sub(x14._super.high, x48.values[decode(1879048178u)].high._super);
eqz(x49);
x17 = x16;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x50: InstOutputBaseStruct;
if ((x5) != 0u) {
x50 = x17;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x50;
}
fn exec_Control0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Control0Layout, global3: u32) -> InstOutputBaseStruct {
// Control0(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:213)
let x4: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Control0Layout__0(layout2));
var x5: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Control0(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:215)
let x6: InstOutputBaseStruct = exec_ControlLoadRootAndNonceChunk0(arg0, arg1_0, lookup_Control0_SuperArm0Layout__super(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))), global3);
// Control0(zirgen/circuit/rv32im/v2/dsl/inst_control.zir:214)
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra0(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra0(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra1(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra1(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra2(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra2(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra3(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra3(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra4(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra4(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra5(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra5(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra6(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra6(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra7(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Control0_SuperArm0Layout__extra7(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra8(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra8(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra9(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra9(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra10(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra10(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra11(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra11(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra12(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra12(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra13(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra13(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra14(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra14(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra15(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra15(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra16(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra16(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra17(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra17(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra18(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra18(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra19(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra19(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra20(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra20(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra21(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra21(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra22(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra22(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra23(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Control0_SuperArm0Layout__extra23(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra24(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra24(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra25(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra25(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra26(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra26(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra27(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra27(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra28(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra28(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra29(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra29(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra30(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra30(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra31(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra31(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra32(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra32(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra33(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra33(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra34(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra34(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra35(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra35(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra36(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra36(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra37(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra37(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra38(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra38(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra39(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Control0_SuperArm0Layout__extra39(lookup_Control0_SuperLayout_arm0(lookup_Control0Layout__super(layout2))))), 0));
x5 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x7: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
x7 = x5;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x7;
}
