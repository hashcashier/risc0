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
fn exec_Reg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
let x2: NondetRegStruct = exec_NondetReg(arg0, layout1);
// Reg(<preamble>:6)
eqz(sub(arg0, x2._super));
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
fn exec_ReadAddr(arg0: NondetRegStruct, arg1_0: Val, layout2: BoundLayout_ReadAddrLayout) -> ReadAddrStruct {
// ReadAddr(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:78)
let x3: GetDataStruct = exec_MemoryRead(arg0, add(arg1_0, 1735917570u), lookup_ReadAddrLayout_addr32(layout2));
// builtin Mul
// ReadAddr(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:79)
let x4: Val = mul(x3._super.high, 1073706872u);
// Div(<preamble>:19)
let x5: Val = mul(x3._super.low, 1073741824u);
return ReadAddrStruct(add(x4, x5));
}
fn back_ShaState(distance0: Index, layout1: BoundLayout_ShaStateLayout) -> ShaStateStruct {
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:20)
let x2: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_stateInAddr(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:21)
let x3: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_stateOutAddr(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:22)
let x4: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_dataAddr(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:23)
let x5: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_count(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:24)
let x6: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_kAddr(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:25)
let x7: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_round(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:26)
let x8: NondetRegStruct = back_Reg(distance0, lookup_ShaStateLayout_nextState(layout1));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:27)
let x9: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(0u)));
let x10: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(268435454u)));
let x11: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(536870908u)));
let x12: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(805306362u)));
let x13: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1073741816u)));
let x14: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1342177270u)));
let x15: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1610612724u)));
let x16: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1879048178u)));
let x17: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(134217711u)));
let x18: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(402653165u)));
let x19: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(671088619u)));
let x20: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(939524073u)));
let x21: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1207959527u)));
let x22: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1476394981u)));
let x23: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1744830435u)));
let x24: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(2013265889u)));
let x25: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(268435422u)));
let x26: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(536870876u)));
let x27: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(805306330u)));
let x28: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1073741784u)));
let x29: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1342177238u)));
let x30: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1610612692u)));
let x31: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1879048146u)));
let x32: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(134217679u)));
let x33: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(402653133u)));
let x34: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(671088587u)));
let x35: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(939524041u)));
let x36: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1207959495u)));
let x37: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1476394949u)));
let x38: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(1744830403u)));
let x39: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(2013265857u)));
let x40: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout1), decode(268435390u)));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:28)
let x41: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(0u)));
let x42: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(268435454u)));
let x43: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(536870908u)));
let x44: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(805306362u)));
let x45: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1073741816u)));
let x46: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1342177270u)));
let x47: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1610612724u)));
let x48: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1879048178u)));
let x49: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(134217711u)));
let x50: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(402653165u)));
let x51: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(671088619u)));
let x52: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(939524073u)));
let x53: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1207959527u)));
let x54: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1476394981u)));
let x55: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1744830435u)));
let x56: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(2013265889u)));
let x57: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(268435422u)));
let x58: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(536870876u)));
let x59: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(805306330u)));
let x60: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1073741784u)));
let x61: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1342177238u)));
let x62: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1610612692u)));
let x63: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1879048146u)));
let x64: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(134217679u)));
let x65: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(402653133u)));
let x66: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(671088587u)));
let x67: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(939524041u)));
let x68: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1207959495u)));
let x69: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1476394949u)));
let x70: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(1744830403u)));
let x71: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(2013265857u)));
let x72: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout1), decode(268435390u)));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:29)
let x73: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(0u)));
let x74: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(268435454u)));
let x75: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(536870908u)));
let x76: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(805306362u)));
let x77: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1073741816u)));
let x78: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1342177270u)));
let x79: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1610612724u)));
let x80: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1879048178u)));
let x81: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(134217711u)));
let x82: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(402653165u)));
let x83: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(671088619u)));
let x84: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(939524073u)));
let x85: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1207959527u)));
let x86: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1476394981u)));
let x87: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1744830435u)));
let x88: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(2013265889u)));
let x89: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(268435422u)));
let x90: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(536870876u)));
let x91: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(805306330u)));
let x92: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1073741784u)));
let x93: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1342177238u)));
let x94: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1610612692u)));
let x95: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1879048146u)));
let x96: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(134217679u)));
let x97: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(402653133u)));
let x98: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(671088587u)));
let x99: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(939524041u)));
let x100: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1207959495u)));
let x101: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1476394949u)));
let x102: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(1744830403u)));
let x103: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(2013265857u)));
let x104: NondetRegStruct = back_NondetReg(distance0, subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout1), decode(268435390u)));
return ShaStateStruct(x2, x3, x4, x5, x6, x7, x8, ShaStateAStruct32Array(ShaStateAStruct(x9), ShaStateAStruct(x10), ShaStateAStruct(x11), ShaStateAStruct(x12), ShaStateAStruct(x13), ShaStateAStruct(x14), ShaStateAStruct(x15), ShaStateAStruct(x16), ShaStateAStruct(x17), ShaStateAStruct(x18), ShaStateAStruct(x19), ShaStateAStruct(x20), ShaStateAStruct(x21), ShaStateAStruct(x22), ShaStateAStruct(x23), ShaStateAStruct(x24), ShaStateAStruct(x25), ShaStateAStruct(x26), ShaStateAStruct(x27), ShaStateAStruct(x28), ShaStateAStruct(x29), ShaStateAStruct(x30), ShaStateAStruct(x31), ShaStateAStruct(x32), ShaStateAStruct(x33), ShaStateAStruct(x34), ShaStateAStruct(x35), ShaStateAStruct(x36), ShaStateAStruct(x37), ShaStateAStruct(x38), ShaStateAStruct(x39), ShaStateAStruct(x40)), ShaStateEStruct32Array(ShaStateEStruct(x41), ShaStateEStruct(x42), ShaStateEStruct(x43), ShaStateEStruct(x44), ShaStateEStruct(x45), ShaStateEStruct(x46), ShaStateEStruct(x47), ShaStateEStruct(x48), ShaStateEStruct(x49), ShaStateEStruct(x50), ShaStateEStruct(x51), ShaStateEStruct(x52), ShaStateEStruct(x53), ShaStateEStruct(x54), ShaStateEStruct(x55), ShaStateEStruct(x56), ShaStateEStruct(x57), ShaStateEStruct(x58), ShaStateEStruct(x59), ShaStateEStruct(x60), ShaStateEStruct(x61), ShaStateEStruct(x62), ShaStateEStruct(x63), ShaStateEStruct(x64), ShaStateEStruct(x65), ShaStateEStruct(x66), ShaStateEStruct(x67), ShaStateEStruct(x68), ShaStateEStruct(x69), ShaStateEStruct(x70), ShaStateEStruct(x71), ShaStateEStruct(x72)), ShaStateWStruct32Array(ShaStateWStruct(x73), ShaStateWStruct(x74), ShaStateWStruct(x75), ShaStateWStruct(x76), ShaStateWStruct(x77), ShaStateWStruct(x78), ShaStateWStruct(x79), ShaStateWStruct(x80), ShaStateWStruct(x81), ShaStateWStruct(x82), ShaStateWStruct(x83), ShaStateWStruct(x84), ShaStateWStruct(x85), ShaStateWStruct(x86), ShaStateWStruct(x87), ShaStateWStruct(x88), ShaStateWStruct(x89), ShaStateWStruct(x90), ShaStateWStruct(x91), ShaStateWStruct(x92), ShaStateWStruct(x93), ShaStateWStruct(x94), ShaStateWStruct(x95), ShaStateWStruct(x96), ShaStateWStruct(x97), ShaStateWStruct(x98), ShaStateWStruct(x99), ShaStateWStruct(x100), ShaStateWStruct(x101), ShaStateWStruct(x102), ShaStateWStruct(x103), ShaStateWStruct(x104)));
}
fn exec_ShaState(arg0: Val32Array, arg1_0: Val32Array, arg2_0: Val32Array, arg3: Val, arg4: Val, arg5: Val, arg6: Val, arg7: Val, arg8: Val, arg9: Val, layout10: BoundLayout_ShaStateLayout) -> ShaStateStruct {
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:20)
let x11: NondetRegStruct = exec_Reg(arg3, lookup_ShaStateLayout_stateInAddr(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:21)
let x12: NondetRegStruct = exec_Reg(arg4, lookup_ShaStateLayout_stateOutAddr(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:22)
let x13: NondetRegStruct = exec_Reg(arg5, lookup_ShaStateLayout_dataAddr(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:23)
let x14: NondetRegStruct = exec_Reg(arg6, lookup_ShaStateLayout_count(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:24)
let x15: NondetRegStruct = exec_Reg(arg7, lookup_ShaStateLayout_kAddr(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:25)
let x16: NondetRegStruct = exec_Reg(arg8, lookup_ShaStateLayout_round(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:26)
let x17: NondetRegStruct = exec_Reg(arg9, lookup_ShaStateLayout_nextState(layout10));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:27)
let x18: NondetRegStruct = exec_NondetReg(arg0[decode(0u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(0u)));
let x19: NondetRegStruct = exec_NondetReg(arg0[decode(268435454u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(268435454u)));
let x20: NondetRegStruct = exec_NondetReg(arg0[decode(536870908u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(536870908u)));
let x21: NondetRegStruct = exec_NondetReg(arg0[decode(805306362u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(805306362u)));
let x22: NondetRegStruct = exec_NondetReg(arg0[decode(1073741816u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1073741816u)));
let x23: NondetRegStruct = exec_NondetReg(arg0[decode(1342177270u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1342177270u)));
let x24: NondetRegStruct = exec_NondetReg(arg0[decode(1610612724u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1610612724u)));
let x25: NondetRegStruct = exec_NondetReg(arg0[decode(1879048178u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1879048178u)));
let x26: NondetRegStruct = exec_NondetReg(arg0[decode(134217711u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(134217711u)));
let x27: NondetRegStruct = exec_NondetReg(arg0[decode(402653165u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(402653165u)));
let x28: NondetRegStruct = exec_NondetReg(arg0[decode(671088619u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(671088619u)));
let x29: NondetRegStruct = exec_NondetReg(arg0[decode(939524073u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(939524073u)));
let x30: NondetRegStruct = exec_NondetReg(arg0[decode(1207959527u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1207959527u)));
let x31: NondetRegStruct = exec_NondetReg(arg0[decode(1476394981u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1476394981u)));
let x32: NondetRegStruct = exec_NondetReg(arg0[decode(1744830435u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1744830435u)));
let x33: NondetRegStruct = exec_NondetReg(arg0[decode(2013265889u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(2013265889u)));
let x34: NondetRegStruct = exec_NondetReg(arg0[decode(268435422u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(268435422u)));
let x35: NondetRegStruct = exec_NondetReg(arg0[decode(536870876u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(536870876u)));
let x36: NondetRegStruct = exec_NondetReg(arg0[decode(805306330u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(805306330u)));
let x37: NondetRegStruct = exec_NondetReg(arg0[decode(1073741784u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1073741784u)));
let x38: NondetRegStruct = exec_NondetReg(arg0[decode(1342177238u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1342177238u)));
let x39: NondetRegStruct = exec_NondetReg(arg0[decode(1610612692u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1610612692u)));
let x40: NondetRegStruct = exec_NondetReg(arg0[decode(1879048146u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1879048146u)));
let x41: NondetRegStruct = exec_NondetReg(arg0[decode(134217679u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(134217679u)));
let x42: NondetRegStruct = exec_NondetReg(arg0[decode(402653133u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(402653133u)));
let x43: NondetRegStruct = exec_NondetReg(arg0[decode(671088587u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(671088587u)));
let x44: NondetRegStruct = exec_NondetReg(arg0[decode(939524041u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(939524041u)));
let x45: NondetRegStruct = exec_NondetReg(arg0[decode(1207959495u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1207959495u)));
let x46: NondetRegStruct = exec_NondetReg(arg0[decode(1476394949u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1476394949u)));
let x47: NondetRegStruct = exec_NondetReg(arg0[decode(1744830403u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(1744830403u)));
let x48: NondetRegStruct = exec_NondetReg(arg0[decode(2013265857u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(2013265857u)));
let x49: NondetRegStruct = exec_NondetReg(arg0[decode(268435390u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_a(layout10), decode(268435390u)));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:28)
let x50: NondetRegStruct = exec_NondetReg(arg1_0[decode(0u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(0u)));
let x51: NondetRegStruct = exec_NondetReg(arg1_0[decode(268435454u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(268435454u)));
let x52: NondetRegStruct = exec_NondetReg(arg1_0[decode(536870908u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(536870908u)));
let x53: NondetRegStruct = exec_NondetReg(arg1_0[decode(805306362u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(805306362u)));
let x54: NondetRegStruct = exec_NondetReg(arg1_0[decode(1073741816u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1073741816u)));
let x55: NondetRegStruct = exec_NondetReg(arg1_0[decode(1342177270u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1342177270u)));
let x56: NondetRegStruct = exec_NondetReg(arg1_0[decode(1610612724u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1610612724u)));
let x57: NondetRegStruct = exec_NondetReg(arg1_0[decode(1879048178u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1879048178u)));
let x58: NondetRegStruct = exec_NondetReg(arg1_0[decode(134217711u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(134217711u)));
let x59: NondetRegStruct = exec_NondetReg(arg1_0[decode(402653165u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(402653165u)));
let x60: NondetRegStruct = exec_NondetReg(arg1_0[decode(671088619u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(671088619u)));
let x61: NondetRegStruct = exec_NondetReg(arg1_0[decode(939524073u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(939524073u)));
let x62: NondetRegStruct = exec_NondetReg(arg1_0[decode(1207959527u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1207959527u)));
let x63: NondetRegStruct = exec_NondetReg(arg1_0[decode(1476394981u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1476394981u)));
let x64: NondetRegStruct = exec_NondetReg(arg1_0[decode(1744830435u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1744830435u)));
let x65: NondetRegStruct = exec_NondetReg(arg1_0[decode(2013265889u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(2013265889u)));
let x66: NondetRegStruct = exec_NondetReg(arg1_0[decode(268435422u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(268435422u)));
let x67: NondetRegStruct = exec_NondetReg(arg1_0[decode(536870876u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(536870876u)));
let x68: NondetRegStruct = exec_NondetReg(arg1_0[decode(805306330u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(805306330u)));
let x69: NondetRegStruct = exec_NondetReg(arg1_0[decode(1073741784u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1073741784u)));
let x70: NondetRegStruct = exec_NondetReg(arg1_0[decode(1342177238u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1342177238u)));
let x71: NondetRegStruct = exec_NondetReg(arg1_0[decode(1610612692u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1610612692u)));
let x72: NondetRegStruct = exec_NondetReg(arg1_0[decode(1879048146u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1879048146u)));
let x73: NondetRegStruct = exec_NondetReg(arg1_0[decode(134217679u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(134217679u)));
let x74: NondetRegStruct = exec_NondetReg(arg1_0[decode(402653133u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(402653133u)));
let x75: NondetRegStruct = exec_NondetReg(arg1_0[decode(671088587u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(671088587u)));
let x76: NondetRegStruct = exec_NondetReg(arg1_0[decode(939524041u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(939524041u)));
let x77: NondetRegStruct = exec_NondetReg(arg1_0[decode(1207959495u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1207959495u)));
let x78: NondetRegStruct = exec_NondetReg(arg1_0[decode(1476394949u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1476394949u)));
let x79: NondetRegStruct = exec_NondetReg(arg1_0[decode(1744830403u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(1744830403u)));
let x80: NondetRegStruct = exec_NondetReg(arg1_0[decode(2013265857u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(2013265857u)));
let x81: NondetRegStruct = exec_NondetReg(arg1_0[decode(268435390u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_e(layout10), decode(268435390u)));
// ShaState(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:29)
let x82: NondetRegStruct = exec_NondetReg(arg2_0[decode(0u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(0u)));
let x83: NondetRegStruct = exec_NondetReg(arg2_0[decode(268435454u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(268435454u)));
let x84: NondetRegStruct = exec_NondetReg(arg2_0[decode(536870908u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(536870908u)));
let x85: NondetRegStruct = exec_NondetReg(arg2_0[decode(805306362u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(805306362u)));
let x86: NondetRegStruct = exec_NondetReg(arg2_0[decode(1073741816u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1073741816u)));
let x87: NondetRegStruct = exec_NondetReg(arg2_0[decode(1342177270u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1342177270u)));
let x88: NondetRegStruct = exec_NondetReg(arg2_0[decode(1610612724u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1610612724u)));
let x89: NondetRegStruct = exec_NondetReg(arg2_0[decode(1879048178u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1879048178u)));
let x90: NondetRegStruct = exec_NondetReg(arg2_0[decode(134217711u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(134217711u)));
let x91: NondetRegStruct = exec_NondetReg(arg2_0[decode(402653165u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(402653165u)));
let x92: NondetRegStruct = exec_NondetReg(arg2_0[decode(671088619u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(671088619u)));
let x93: NondetRegStruct = exec_NondetReg(arg2_0[decode(939524073u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(939524073u)));
let x94: NondetRegStruct = exec_NondetReg(arg2_0[decode(1207959527u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1207959527u)));
let x95: NondetRegStruct = exec_NondetReg(arg2_0[decode(1476394981u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1476394981u)));
let x96: NondetRegStruct = exec_NondetReg(arg2_0[decode(1744830435u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1744830435u)));
let x97: NondetRegStruct = exec_NondetReg(arg2_0[decode(2013265889u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(2013265889u)));
let x98: NondetRegStruct = exec_NondetReg(arg2_0[decode(268435422u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(268435422u)));
let x99: NondetRegStruct = exec_NondetReg(arg2_0[decode(536870876u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(536870876u)));
let x100: NondetRegStruct = exec_NondetReg(arg2_0[decode(805306330u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(805306330u)));
let x101: NondetRegStruct = exec_NondetReg(arg2_0[decode(1073741784u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1073741784u)));
let x102: NondetRegStruct = exec_NondetReg(arg2_0[decode(1342177238u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1342177238u)));
let x103: NondetRegStruct = exec_NondetReg(arg2_0[decode(1610612692u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1610612692u)));
let x104: NondetRegStruct = exec_NondetReg(arg2_0[decode(1879048146u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1879048146u)));
let x105: NondetRegStruct = exec_NondetReg(arg2_0[decode(134217679u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(134217679u)));
let x106: NondetRegStruct = exec_NondetReg(arg2_0[decode(402653133u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(402653133u)));
let x107: NondetRegStruct = exec_NondetReg(arg2_0[decode(671088587u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(671088587u)));
let x108: NondetRegStruct = exec_NondetReg(arg2_0[decode(939524041u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(939524041u)));
let x109: NondetRegStruct = exec_NondetReg(arg2_0[decode(1207959495u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1207959495u)));
let x110: NondetRegStruct = exec_NondetReg(arg2_0[decode(1476394949u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1476394949u)));
let x111: NondetRegStruct = exec_NondetReg(arg2_0[decode(1744830403u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(1744830403u)));
let x112: NondetRegStruct = exec_NondetReg(arg2_0[decode(2013265857u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(2013265857u)));
let x113: NondetRegStruct = exec_NondetReg(arg2_0[decode(268435390u)], subscript_NondetRegLayout32LayoutArray(lookup_ShaStateLayout_w(layout10), decode(268435390u)));
return ShaStateStruct(x11, x12, x13, x14, x15, x16, x17, ShaStateAStruct32Array(ShaStateAStruct(x18), ShaStateAStruct(x19), ShaStateAStruct(x20), ShaStateAStruct(x21), ShaStateAStruct(x22), ShaStateAStruct(x23), ShaStateAStruct(x24), ShaStateAStruct(x25), ShaStateAStruct(x26), ShaStateAStruct(x27), ShaStateAStruct(x28), ShaStateAStruct(x29), ShaStateAStruct(x30), ShaStateAStruct(x31), ShaStateAStruct(x32), ShaStateAStruct(x33), ShaStateAStruct(x34), ShaStateAStruct(x35), ShaStateAStruct(x36), ShaStateAStruct(x37), ShaStateAStruct(x38), ShaStateAStruct(x39), ShaStateAStruct(x40), ShaStateAStruct(x41), ShaStateAStruct(x42), ShaStateAStruct(x43), ShaStateAStruct(x44), ShaStateAStruct(x45), ShaStateAStruct(x46), ShaStateAStruct(x47), ShaStateAStruct(x48), ShaStateAStruct(x49)), ShaStateEStruct32Array(ShaStateEStruct(x50), ShaStateEStruct(x51), ShaStateEStruct(x52), ShaStateEStruct(x53), ShaStateEStruct(x54), ShaStateEStruct(x55), ShaStateEStruct(x56), ShaStateEStruct(x57), ShaStateEStruct(x58), ShaStateEStruct(x59), ShaStateEStruct(x60), ShaStateEStruct(x61), ShaStateEStruct(x62), ShaStateEStruct(x63), ShaStateEStruct(x64), ShaStateEStruct(x65), ShaStateEStruct(x66), ShaStateEStruct(x67), ShaStateEStruct(x68), ShaStateEStruct(x69), ShaStateEStruct(x70), ShaStateEStruct(x71), ShaStateEStruct(x72), ShaStateEStruct(x73), ShaStateEStruct(x74), ShaStateEStruct(x75), ShaStateEStruct(x76), ShaStateEStruct(x77), ShaStateEStruct(x78), ShaStateEStruct(x79), ShaStateEStruct(x80), ShaStateEStruct(x81)), ShaStateWStruct32Array(ShaStateWStruct(x82), ShaStateWStruct(x83), ShaStateWStruct(x84), ShaStateWStruct(x85), ShaStateWStruct(x86), ShaStateWStruct(x87), ShaStateWStruct(x88), ShaStateWStruct(x89), ShaStateWStruct(x90), ShaStateWStruct(x91), ShaStateWStruct(x92), ShaStateWStruct(x93), ShaStateWStruct(x94), ShaStateWStruct(x95), ShaStateWStruct(x96), ShaStateWStruct(x97), ShaStateWStruct(x98), ShaStateWStruct(x99), ShaStateWStruct(x100), ShaStateWStruct(x101), ShaStateWStruct(x102), ShaStateWStruct(x103), ShaStateWStruct(x104), ShaStateWStruct(x105), ShaStateWStruct(x106), ShaStateWStruct(x107), ShaStateWStruct(x108), ShaStateWStruct(x109), ShaStateWStruct(x110), ShaStateWStruct(x111), ShaStateWStruct(x112), ShaStateWStruct(x113)));
}
fn exec_ShaEcall(arg0: NondetRegStruct, layout1: BoundLayout_ShaEcallLayout) -> ShaStateStruct {
// Log(<preamble>:22)
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:44)
extern_noop();
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:46)
let x2: ReadAddrStruct = exec_ReadAddr(arg0, 671088619u, lookup_ShaEcallLayout_stateInAddr(layout1));
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:47)
let x3: ReadAddrStruct = exec_ReadAddr(arg0, 939524073u, lookup_ShaEcallLayout_stateOutAddr(layout1));
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:48)
let x4: ReadAddrStruct = exec_ReadAddr(arg0, 1207959527u, lookup_ShaEcallLayout_dataAddr(layout1));
// Log(<preamble>:22)
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:49)
extern_noop();
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:50)
let x5: GetDataStruct = exec_MemoryRead(arg0, 1199046630u, lookup_ShaEcallLayout__0(layout1));
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:51)
let x6: ReadAddrStruct = exec_ReadAddr(arg0, 1744830435u, lookup_ShaEcallLayout_kAddr(layout1));
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:53)
let x7: Val32Array = Val32Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
// ShaEcall(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:52)
let x8: ShaStateStruct = exec_ShaState(x7, x7, x7, x2._super, x3._super, x4._super, x5._super.low, x6._super, 0u, 805306298u, lookup_ShaEcallLayout__super(layout1));
return x8;
}
fn exec_Sha0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Sha0Layout) -> InstOutputBaseStruct {
// Sha0(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:225)
let x3: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Sha0Layout__0(layout2));
// Sha0(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:226)
let x4: Val = sub(arg1_0.state, add(arg1_0.minor, 536870844u));
eqz(x4);
var x5: ShaStateStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Sha0(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:229)
let x6: ShaStateStruct = exec_ShaEcall(arg0, lookup_Sha0StateLayout_arm0(lookup_Sha0Layout_stateRedef(layout2)));
x5 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x7: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Sha0(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:228)
let x8: ShaStateStruct = back_ShaState(0, lookup_Sha0StateLayout__super(lookup_Sha0Layout_stateRedef(layout2)));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// Sha0(zirgen/circuit/rv32im/v2/dsl/inst_sha.zir:238)
let x9: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
// InstOutputBase(zirgen/circuit/rv32im/v2/dsl/inst.zir:78)
let x10: InstOutputBaseStruct = InstOutputBaseStruct(arg1_0.pcU32, x8.nextState._super, arg1_0.mode, x9);
x7 = x10;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x7;
}
