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
fn back_NondetBitReg(distance0: Index, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
// NondetBitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:13)
let x2: NondetRegStruct = back_NondetReg(distance0, layout1);
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
fn back_BitReg(distance0: Index, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
// BitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:19)
let x2: NondetRegStruct = back_NondetBitReg(distance0, layout1);
return x2;
}
fn exec_BitReg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
let x2: NondetRegStruct = exec_NondetBitReg(arg0, layout1);
// BitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:20)
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
fn back_BigIntState(distance0: Index, layout1: BoundLayout_BigIntStateLayout) -> BigIntStateStruct {
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:14)
let x2: NondetRegStruct = back_BitReg(distance0, lookup_BigIntStateLayout_isEcall(layout1));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:15)
let x3: NondetRegStruct = back_BitReg(distance0, lookup_BigIntStateLayout_mode(layout1));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:16)
let x4: NondetRegStruct = back_Reg(distance0, lookup_BigIntStateLayout_pc(layout1));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:17)
let x5: NondetRegStruct = back_Reg(distance0, lookup_BigIntStateLayout_polyOp(layout1));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:18)
let x6: NondetRegStruct = back_Reg(distance0, lookup_BigIntStateLayout_coeff(layout1));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:19)
let x7: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(0u)));
let x8: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(268435454u)));
let x9: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(536870908u)));
let x10: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(805306362u)));
let x11: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1073741816u)));
let x12: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1342177270u)));
let x13: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1610612724u)));
let x14: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1879048178u)));
let x15: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(134217711u)));
let x16: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(402653165u)));
let x17: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(671088619u)));
let x18: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(939524073u)));
let x19: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1207959527u)));
let x20: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1476394981u)));
let x21: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(1744830435u)));
let x22: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout1), decode(2013265889u)));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:20)
let x23: NondetRegStruct = back_Reg(distance0, lookup_BigIntStateLayout_nextState(layout1));
return BigIntStateStruct(x2, x3, x4, x5, x6, NondetRegStruct16Array(x7, x8, x9, x10, x11, x12, x13, x14, x15, x16, x17, x18, x19, x20, x21, x22), x23);
}
fn exec_BigIntState(arg0: Val, arg1_0: Val, arg2_0: Val, arg3: Val, arg4: Val, arg5: Val16Array, arg6: Val, layout7: BoundLayout_BigIntStateLayout) -> BigIntStateStruct {
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:14)
let x8: NondetRegStruct = exec_BitReg(arg0, lookup_BigIntStateLayout_isEcall(layout7));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:15)
let x9: NondetRegStruct = exec_BitReg(arg1_0, lookup_BigIntStateLayout_mode(layout7));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:16)
let x10: NondetRegStruct = exec_Reg(arg2_0, lookup_BigIntStateLayout_pc(layout7));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:17)
let x11: NondetRegStruct = exec_Reg(arg3, lookup_BigIntStateLayout_polyOp(layout7));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:18)
let x12: NondetRegStruct = exec_Reg(arg4, lookup_BigIntStateLayout_coeff(layout7));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:19)
let x13: NondetRegStruct = exec_Reg(arg5[decode(0u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(0u)));
let x14: NondetRegStruct = exec_Reg(arg5[decode(268435454u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(268435454u)));
let x15: NondetRegStruct = exec_Reg(arg5[decode(536870908u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(536870908u)));
let x16: NondetRegStruct = exec_Reg(arg5[decode(805306362u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(805306362u)));
let x17: NondetRegStruct = exec_Reg(arg5[decode(1073741816u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1073741816u)));
let x18: NondetRegStruct = exec_Reg(arg5[decode(1342177270u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1342177270u)));
let x19: NondetRegStruct = exec_Reg(arg5[decode(1610612724u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1610612724u)));
let x20: NondetRegStruct = exec_Reg(arg5[decode(1879048178u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1879048178u)));
let x21: NondetRegStruct = exec_Reg(arg5[decode(134217711u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(134217711u)));
let x22: NondetRegStruct = exec_Reg(arg5[decode(402653165u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(402653165u)));
let x23: NondetRegStruct = exec_Reg(arg5[decode(671088619u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(671088619u)));
let x24: NondetRegStruct = exec_Reg(arg5[decode(939524073u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(939524073u)));
let x25: NondetRegStruct = exec_Reg(arg5[decode(1207959527u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1207959527u)));
let x26: NondetRegStruct = exec_Reg(arg5[decode(1476394981u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1476394981u)));
let x27: NondetRegStruct = exec_Reg(arg5[decode(1744830435u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(1744830435u)));
let x28: NondetRegStruct = exec_Reg(arg5[decode(2013265889u)], subscript_NondetRegLayout16LayoutArray(lookup_BigIntStateLayout_bytes(layout7), decode(2013265889u)));
// BigIntState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:20)
let x29: NondetRegStruct = exec_Reg(arg6, lookup_BigIntStateLayout_nextState(layout7));
return BigIntStateStruct(x8, x9, x10, x11, x12, NondetRegStruct16Array(x13, x14, x15, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28), x29);
}
fn exec_BigIntEcall(arg0: NondetRegStruct, layout1: BoundLayout_BigIntEcallLayout) -> BigIntStateStruct {
// BigIntEcall(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:41)
let x2: GetDataStruct = exec_MemoryRead(arg0, 1064828919u, lookup_BigIntEcallLayout_mode(layout1));
// BigIntEcall(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:42)
eqz(x2._super.high);
// BigIntEcall(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:44)
let x3: ReadAddrStruct = exec_ReadAddr(arg0, 1879048178u, lookup_BigIntEcallLayout_pc(layout1));
// BigIntEcall(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:46)
let x4: BigIntStateStruct = exec_BigIntState(268435454u, x2._super.low, sub(x3._super, 268435454u), 0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u), 939524009u, lookup_BigIntEcallLayout__super(layout1));
return x4;
}
fn exec_BigInt0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_BigInt0Layout) -> InstOutputBaseStruct {
// Log(<preamble>:22)
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:170)
extern_noop();
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:171)
let x3: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_BigInt0Layout__0(layout2));
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:172)
let x4: Val = sub(arg1_0.state, add(arg1_0.minor, 671088555u));
eqz(x4);
var x5: BigIntStateStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:175)
let x6: BigIntStateStruct = exec_BigIntEcall(arg0, lookup_BigInt0StateArm0Layout__super(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))));
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:174)
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra0(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra0(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra1(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra1(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra2(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra2(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra3(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra3(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra4(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra4(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra5(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra5(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra6(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra6(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra7(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_BigInt0StateArm0Layout__extra7(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra8(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra8(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra9(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra9(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra10(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra10(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra11(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_BigInt0StateArm0Layout__extra11(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra12(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra12(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra13(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra13(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra14(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra14(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra15(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra15(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra16(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra16(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra17(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra17(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra18(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra18(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra19(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra19(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra20(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra20(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra21(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra21(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra22(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra22(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra23(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra23(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra24(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra24(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra25(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra25(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra26(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra26(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra27(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra27(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra28(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra28(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra29(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_BigInt0StateArm0Layout__extra29(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra30(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra30(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra31(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra31(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra32(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra32(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra33(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_BigInt0StateArm0Layout__extra33(lookup_BigInt0StateLayout_arm0(lookup_BigInt0Layout_stateRedef(layout2))))), 0));
x5 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x7: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
let x8: BigIntStateStruct = back_BigIntState(0, lookup_BigInt0StateLayout__super(lookup_BigInt0Layout_stateRedef(layout2)));
let x9: Val16Array = Val16Array(x8.bytes[decode(0u)]._super, x8.bytes[decode(268435454u)]._super, x8.bytes[decode(536870908u)]._super, x8.bytes[decode(805306362u)]._super, x8.bytes[decode(1073741816u)]._super, x8.bytes[decode(1342177270u)]._super, x8.bytes[decode(1610612724u)]._super, x8.bytes[decode(1879048178u)]._super, x8.bytes[decode(134217711u)]._super, x8.bytes[decode(402653165u)]._super, x8.bytes[decode(671088619u)]._super, x8.bytes[decode(939524073u)]._super, x8.bytes[decode(1207959527u)]._super, x8.bytes[decode(1476394981u)]._super, x8.bytes[decode(1744830435u)]._super, x8.bytes[decode(2013265889u)]._super);
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:184)
let x10: BigIntTopStateStruct = BigIntTopStateStruct(x8.polyOp._super, x8.coeff._super, x9);
// InstOutputBase(zirgen/circuit/rv32im/v2/dsl/inst.zir:78)
// BigInt0(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:185)
let x11: InstOutputBaseStruct = InstOutputBaseStruct(arg1_0.pcU32, x8.nextState._super, arg1_0.mode, x10);
x7 = x11;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x7;
}
