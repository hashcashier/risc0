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
fn back_NondetExtReg(distance0: Index, layout1: BoundLayout_NondetExtRegLayout) -> NondetExtRegStruct {
// builtin NondetExtReg
let x2: NondetExtRegStruct = NondetExtRegStruct(load_ext(lookup_NondetExtRegLayout__super(layout1), distance0));
return x2;
}
fn exec_NondetExtReg(arg0: ExtVal, layout1: BoundLayout_NondetExtRegLayout) -> NondetExtRegStruct {
store_ext(lookup_NondetExtRegLayout__super(layout1), arg0);
let x2: NondetExtRegStruct = NondetExtRegStruct(load_ext(lookup_NondetExtRegLayout__super(layout1), 0));
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
fn back_ExtReg(distance0: Index, layout1: BoundLayout_NondetExtRegLayout) -> NondetExtRegStruct {
// ExtReg(<preamble>:12)
let x2: NondetExtRegStruct = back_NondetExtReg(distance0, layout1);
return x2;
}
fn exec_ExtReg(arg0: ExtVal, layout1: BoundLayout_NondetExtRegLayout) -> NondetExtRegStruct {
let x2: NondetExtRegStruct = exec_NondetExtReg(arg0, layout1);
// builtin EqzExt
// ExtReg(<preamble>:13)
eqz_ext(ext_sub(x2._super, arg0));
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
fn back_PoseidonState(distance0: Index, layout1: BoundLayout_PoseidonStateLayout) -> PoseidonStateStruct {
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:34)
let x2: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_hasState(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:35)
let x3: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_stateAddr(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:36)
let x4: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_bufOutAddr(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:37)
let x5: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_isElem(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:38)
let x6: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_checkOut(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:39)
let x7: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_loadTxType(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:41)
let x8: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_nextState(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:42)
let x9: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_subState(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:43)
let x10: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_bufInAddr(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:44)
let x11: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_count(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:45)
let x12: NondetRegStruct = back_Reg(distance0, lookup_PoseidonStateLayout_mode(layout1));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:47)
let x13: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(0u)));
let x14: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(268435454u)));
let x15: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(536870908u)));
let x16: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(805306362u)));
let x17: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1073741816u)));
let x18: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1342177270u)));
let x19: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1610612724u)));
let x20: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1879048178u)));
let x21: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(134217711u)));
let x22: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(402653165u)));
let x23: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(671088619u)));
let x24: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(939524073u)));
let x25: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1207959527u)));
let x26: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1476394981u)));
let x27: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1744830435u)));
let x28: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(2013265889u)));
let x29: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(268435422u)));
let x30: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(536870876u)));
let x31: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(805306330u)));
let x32: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1073741784u)));
let x33: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1342177238u)));
let x34: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1610612692u)));
let x35: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(1879048146u)));
let x36: NondetRegStruct = back_Reg(distance0, subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout1), decode(134217679u)));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:48)
let x37: NondetExtRegStruct = back_ExtReg(distance0, lookup_PoseidonStateLayout_zcheck(layout1));
return PoseidonStateStruct(x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, NondetRegStruct24Array(x13, x14, x15, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, x30, x31, x32, x33, x34, x35, x36), x37);
}
fn exec_PoseidonState(arg0: PoseidonOpDefStruct, arg1_0: Val, arg2_0: Val, arg3: Val, arg4: Val, arg5: Val, arg6: Val24Array, arg7: ExtVal, layout8: BoundLayout_PoseidonStateLayout) -> PoseidonStateStruct {
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:34)
let x9: NondetRegStruct = exec_Reg(arg0.hasState, lookup_PoseidonStateLayout_hasState(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:35)
let x10: NondetRegStruct = exec_Reg(arg0.stateAddr, lookup_PoseidonStateLayout_stateAddr(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:36)
let x11: NondetRegStruct = exec_Reg(arg0.bufOutAddr, lookup_PoseidonStateLayout_bufOutAddr(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:37)
let x12: NondetRegStruct = exec_Reg(arg0.isElem, lookup_PoseidonStateLayout_isElem(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:38)
let x13: NondetRegStruct = exec_Reg(arg0.checkOut, lookup_PoseidonStateLayout_checkOut(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:39)
let x14: NondetRegStruct = exec_Reg(arg0.loadTxType, lookup_PoseidonStateLayout_loadTxType(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:41)
let x15: NondetRegStruct = exec_Reg(arg1_0, lookup_PoseidonStateLayout_nextState(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:42)
let x16: NondetRegStruct = exec_Reg(arg2_0, lookup_PoseidonStateLayout_subState(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:43)
let x17: NondetRegStruct = exec_Reg(arg3, lookup_PoseidonStateLayout_bufInAddr(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:44)
let x18: NondetRegStruct = exec_Reg(arg4, lookup_PoseidonStateLayout_count(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:45)
let x19: NondetRegStruct = exec_Reg(arg5, lookup_PoseidonStateLayout_mode(layout8));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:47)
let x20: NondetRegStruct = exec_Reg(arg6[decode(0u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(0u)));
let x21: NondetRegStruct = exec_Reg(arg6[decode(268435454u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(268435454u)));
let x22: NondetRegStruct = exec_Reg(arg6[decode(536870908u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(536870908u)));
let x23: NondetRegStruct = exec_Reg(arg6[decode(805306362u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(805306362u)));
let x24: NondetRegStruct = exec_Reg(arg6[decode(1073741816u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1073741816u)));
let x25: NondetRegStruct = exec_Reg(arg6[decode(1342177270u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1342177270u)));
let x26: NondetRegStruct = exec_Reg(arg6[decode(1610612724u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1610612724u)));
let x27: NondetRegStruct = exec_Reg(arg6[decode(1879048178u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1879048178u)));
let x28: NondetRegStruct = exec_Reg(arg6[decode(134217711u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(134217711u)));
let x29: NondetRegStruct = exec_Reg(arg6[decode(402653165u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(402653165u)));
let x30: NondetRegStruct = exec_Reg(arg6[decode(671088619u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(671088619u)));
let x31: NondetRegStruct = exec_Reg(arg6[decode(939524073u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(939524073u)));
let x32: NondetRegStruct = exec_Reg(arg6[decode(1207959527u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1207959527u)));
let x33: NondetRegStruct = exec_Reg(arg6[decode(1476394981u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1476394981u)));
let x34: NondetRegStruct = exec_Reg(arg6[decode(1744830435u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1744830435u)));
let x35: NondetRegStruct = exec_Reg(arg6[decode(2013265889u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(2013265889u)));
let x36: NondetRegStruct = exec_Reg(arg6[decode(268435422u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(268435422u)));
let x37: NondetRegStruct = exec_Reg(arg6[decode(536870876u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(536870876u)));
let x38: NondetRegStruct = exec_Reg(arg6[decode(805306330u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(805306330u)));
let x39: NondetRegStruct = exec_Reg(arg6[decode(1073741784u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1073741784u)));
let x40: NondetRegStruct = exec_Reg(arg6[decode(1342177238u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1342177238u)));
let x41: NondetRegStruct = exec_Reg(arg6[decode(1610612692u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1610612692u)));
let x42: NondetRegStruct = exec_Reg(arg6[decode(1879048146u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(1879048146u)));
let x43: NondetRegStruct = exec_Reg(arg6[decode(134217679u)], subscript_NondetRegLayout24LayoutArray(lookup_PoseidonStateLayout_inner(layout8), decode(134217679u)));
// PoseidonState(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:48)
let x44: NondetExtRegStruct = exec_ExtReg(arg7, lookup_PoseidonStateLayout_zcheck(layout8));
return PoseidonStateStruct(x9, x10, x11, x12, x13, x14, x15, x16, x17, x18, x19, NondetRegStruct24Array(x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, x30, x31, x32, x33, x34, x35, x36, x37, x38, x39, x40, x41, x42, x43), x44);
}
fn exec_PoseidonPagingEntry(arg0: NondetRegStruct, arg1_0: Val, layout2: BoundLayout_PoseidonStateLayout) -> PoseidonStateStruct {
// builtin Mul
// Div(<preamble>:19)
// PoseidonPagingEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:110)
let x3: Val = mul(arg1_0, 760567125u);
// PoseidonPagingEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:114)
let x4: Val = mul(sub(268435454u, x3), 1726934769u);
// PoseidonOpDef(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:8)
// PoseidonPagingEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:111)
let x5: PoseidonOpDefStruct = PoseidonOpDefStruct(0u, 0u, add(mul(x3, 796358521u), x4), 268435454u, 268435454u, 268435454u);
// PoseidonPagingEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:119)
let x6: PoseidonStateStruct = exec_PoseidonState(x5, 1879048146u, 0u, 0u, 0u, arg1_0, Val24Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u), layout2);
return x6;
}
fn exec_PoseidonEntryChunk0(arg0: NondetRegStruct, arg1_0: ValU32Struct, arg2_0: Val, layout3: BoundLayout_PoseidonEntryLayout) -> PoseidonStateStruct {
// PoseidonEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:131)
let x4: NondetRegStruct = exec_IsZero(add(arg1_0.low, arg1_0.high), lookup_PoseidonEntryLayout_pcZero(layout3));
var x5: PoseidonStateStruct;
if ((x4._super) != 0u) {
// PoseidonEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:133)
let x6: PoseidonStateStruct = exec_PoseidonPagingEntry(arg0, arg2_0, lookup_PoseidonEntry_SuperArm0Layout__super(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))));
// PoseidonEntry(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:132)
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra0(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra0(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra1(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra1(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra2(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra2(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra3(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra3(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra4(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra4(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra5(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra5(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra6(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra6(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra7(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra7(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra8(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra8(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra9(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra9(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra10(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra10(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra11(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_PoseidonEntry_SuperArm0Layout__extra11(lookup_PoseidonEntry_SuperLayout_arm0(lookup_PoseidonEntryLayout__super(layout3))))), 0));
x5 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x7: PoseidonStateStruct;
if ((x4._super) != 0u) {
let x8: PoseidonStateStruct = back_PoseidonState(0, lookup_PoseidonEntry_SuperLayout__super(lookup_PoseidonEntryLayout__super(layout3)));
x7 = x8;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x7;
}
fn exec_Poseidon0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Poseidon0Layout, global3: u32) -> InstOutputBaseStruct {
// Poseidon0(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:469)
let x4: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Poseidon0Layout__0(layout2));
// Poseidon0(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:470)
let x5: Val = sub(arg1_0.state, add(arg1_0.minor, 268435422u));
eqz(x5);
var x6: PoseidonStateStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Poseidon0(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:473)
let x7: PoseidonStateStruct = exec_PoseidonEntryChunk0(arg0, arg1_0.pcU32, arg1_0.mode, lookup_Poseidon0StateArm0Layout__super(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))));
// Poseidon0(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:472)
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra0(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra0(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra1(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra1(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra2(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra2(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra3(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra3(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra4(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra4(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra5(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra5(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra6(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra6(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra7(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_Poseidon0StateArm0Layout__extra7(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra8(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra8(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra9(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra9(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra10(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra10(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra11(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_Poseidon0StateArm0Layout__extra11(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra12(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra12(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra13(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra13(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra14(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra14(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra15(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra15(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra16(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra16(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra17(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra17(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra18(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra18(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra19(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra19(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra20(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra20(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra21(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra21(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra22(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra22(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra23(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra23(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra24(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra24(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra25(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra25(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra26(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra26(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra27(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra27(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra28(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra28(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra29(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra29(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra30(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra30(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra31(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra31(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra32(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra32(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra33(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra33(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra34(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra34(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra35(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Poseidon0StateArm0Layout__extra35(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Poseidon0StateArm0Layout__extra36(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Poseidon0StateArm0Layout__extra36(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Poseidon0StateArm0Layout__extra37(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Poseidon0StateArm0Layout__extra37(lookup_Poseidon0StateLayout_arm0(lookup_Poseidon0Layout_stateRedef(layout2))))), 0));
x6 = x7;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x8: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
let x9: PoseidonStateStruct = back_PoseidonState(0, lookup_Poseidon0StateLayout__super(lookup_Poseidon0Layout_stateRedef(layout2)));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// Poseidon0(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:482)
let x10: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
// InstOutputBase(zirgen/circuit/rv32im/v2/dsl/inst.zir:78)
let x11: InstOutputBaseStruct = InstOutputBaseStruct(arg1_0.pcU32, x9.nextState._super, x9.mode._super, x10);
x8 = x11;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x8;
}
