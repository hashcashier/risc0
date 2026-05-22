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
fn exec_OneHot_8_(arg0: Val, layout1: BoundLayout_OneHot_8_Layout) -> OneHot_8_Struct {
// builtin Isz
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:7)
let x2: Val = isz(sub(0u, arg0));
let x3: NondetRegStruct = exec_NondetBitReg(x2, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(0u)));
// builtin Isz
let x4: Val = isz(sub(268435454u, arg0));
let x5: NondetRegStruct = exec_NondetBitReg(x4, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(268435454u)));
// builtin Isz
let x6: Val = isz(sub(536870908u, arg0));
let x7: NondetRegStruct = exec_NondetBitReg(x6, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(536870908u)));
// builtin Isz
let x8: Val = isz(sub(805306362u, arg0));
let x9: NondetRegStruct = exec_NondetBitReg(x8, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(805306362u)));
// builtin Isz
let x10: Val = isz(sub(1073741816u, arg0));
let x11: NondetRegStruct = exec_NondetBitReg(x10, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(1073741816u)));
// builtin Isz
let x12: Val = isz(sub(1342177270u, arg0));
let x13: NondetRegStruct = exec_NondetBitReg(x12, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(1342177270u)));
// builtin Isz
let x14: Val = isz(sub(1610612724u, arg0));
let x15: NondetRegStruct = exec_NondetBitReg(x14, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(1610612724u)));
// builtin Isz
let x16: Val = isz(sub(1879048178u, arg0));
let x17: NondetRegStruct = exec_NondetBitReg(x16, subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(layout1), decode(1879048178u)));
// builtin Add
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:9)
let x18: Val = add(add(x3._super, x5._super), x7._super);
let x19: Val = add(add(add(x18, x9._super), x11._super), x13._super);
let x20: Val = sub(add(add(x19, x15._super), x17._super), 268435454u);
eqz(x20);
// builtin Add
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:11)
let x21: Val = add(x5._super, mul(x7._super, 536870908u));
let x22: Val = add(add(x21, mul(x9._super, 805306362u)), mul(x11._super, 1073741816u));
let x23: Val = add(add(x22, mul(x13._super, 1342177270u)), mul(x15._super, 1610612724u));
eqz(sub(add(x23, mul(x17._super, 1879048178u)), arg0));
return OneHot_8_Struct(NondetRegStruct8Array(x3, x5, x7, x9, x11, x13, x15, x17));
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
fn exec_SBox(arg0: Val, layout1: BoundLayout_SBoxLayout) -> NondetRegStruct {
// SBox(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:25)
let x2: NondetRegStruct = exec_Reg(mul(mul(arg0, arg0), arg0), lookup_SBoxLayout_cubed(layout1));
// builtin Mul
// SBox(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:26)
let x3: Val = mul(mul(x2._super, x2._super), arg0);
let x4: NondetRegStruct = exec_Reg(x3, lookup_SBoxLayout__super(layout1));
return x4;
}
fn exec_DoExtRound(arg0: Val24Array, arg1_0: Val24Array, layout2: BoundLayout_DoExtRoundLayout) -> MultiplyByMExtStruct {
// DoExtRound(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:103)
let x3: NondetRegStruct = exec_SBox(add(arg0[decode(0u)], arg1_0[decode(0u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(0u)));
let x4: NondetRegStruct = exec_SBox(add(arg0[decode(268435454u)], arg1_0[decode(268435454u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(268435454u)));
let x5: NondetRegStruct = exec_SBox(add(arg0[decode(536870908u)], arg1_0[decode(536870908u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(536870908u)));
let x6: NondetRegStruct = exec_SBox(add(arg0[decode(805306362u)], arg1_0[decode(805306362u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(805306362u)));
let x7: NondetRegStruct = exec_SBox(add(arg0[decode(1073741816u)], arg1_0[decode(1073741816u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1073741816u)));
let x8: NondetRegStruct = exec_SBox(add(arg0[decode(1342177270u)], arg1_0[decode(1342177270u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1342177270u)));
let x9: NondetRegStruct = exec_SBox(add(arg0[decode(1610612724u)], arg1_0[decode(1610612724u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1610612724u)));
let x10: NondetRegStruct = exec_SBox(add(arg0[decode(1879048178u)], arg1_0[decode(1879048178u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1879048178u)));
let x11: NondetRegStruct = exec_SBox(add(arg0[decode(134217711u)], arg1_0[decode(134217711u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(134217711u)));
let x12: NondetRegStruct = exec_SBox(add(arg0[decode(402653165u)], arg1_0[decode(402653165u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(402653165u)));
let x13: NondetRegStruct = exec_SBox(add(arg0[decode(671088619u)], arg1_0[decode(671088619u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(671088619u)));
let x14: NondetRegStruct = exec_SBox(add(arg0[decode(939524073u)], arg1_0[decode(939524073u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(939524073u)));
let x15: NondetRegStruct = exec_SBox(add(arg0[decode(1207959527u)], arg1_0[decode(1207959527u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1207959527u)));
let x16: NondetRegStruct = exec_SBox(add(arg0[decode(1476394981u)], arg1_0[decode(1476394981u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1476394981u)));
let x17: NondetRegStruct = exec_SBox(add(arg0[decode(1744830435u)], arg1_0[decode(1744830435u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1744830435u)));
let x18: NondetRegStruct = exec_SBox(add(arg0[decode(2013265889u)], arg1_0[decode(2013265889u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(2013265889u)));
let x19: NondetRegStruct = exec_SBox(add(arg0[decode(268435422u)], arg1_0[decode(268435422u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(268435422u)));
let x20: NondetRegStruct = exec_SBox(add(arg0[decode(536870876u)], arg1_0[decode(536870876u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(536870876u)));
let x21: NondetRegStruct = exec_SBox(add(arg0[decode(805306330u)], arg1_0[decode(805306330u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(805306330u)));
let x22: NondetRegStruct = exec_SBox(add(arg0[decode(1073741784u)], arg1_0[decode(1073741784u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1073741784u)));
let x23: NondetRegStruct = exec_SBox(add(arg0[decode(1342177238u)], arg1_0[decode(1342177238u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1342177238u)));
let x24: NondetRegStruct = exec_SBox(add(arg0[decode(1610612692u)], arg1_0[decode(1610612692u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1610612692u)));
let x25: NondetRegStruct = exec_SBox(add(arg0[decode(1879048146u)], arg1_0[decode(1879048146u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(1879048146u)));
let x26: NondetRegStruct = exec_SBox(add(arg0[decode(134217679u)], arg1_0[decode(134217679u)]), subscript_SBoxLayout24LayoutArray(lookup_DoExtRoundLayout__1(layout2), decode(134217679u)));
// builtin Add
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:42)
// MultiplyByMExt(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:61)
let x27: Val = add(x3._super, x4._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:43)
let x28: Val = add(x5._super, x6._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:44)
let x29: Val = add(mul(x4._super, 536870908u), x28);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:45)
let x30: Val = add(mul(x6._super, 536870908u), x27);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:46)
let x31: Val = add(mul(x28, 1073741816u), x30);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:47)
let x32: Val = add(mul(x27, 1073741816u), x29);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:48)
let x33: Val = add(x30, x32);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:49)
let x34: Val = add(x29, x31);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:42)
let x35: Val = add(x7._super, x8._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:43)
let x36: Val = add(x9._super, x10._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:44)
let x37: Val = add(mul(x8._super, 536870908u), x36);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:45)
let x38: Val = add(mul(x10._super, 536870908u), x35);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:46)
let x39: Val = add(mul(x36, 1073741816u), x38);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:47)
let x40: Val = add(mul(x35, 1073741816u), x37);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:48)
let x41: Val = add(x38, x40);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:49)
let x42: Val = add(x37, x39);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:42)
let x43: Val = add(x11._super, x12._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:43)
let x44: Val = add(x13._super, x14._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:44)
let x45: Val = add(mul(x12._super, 536870908u), x44);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:45)
let x46: Val = add(mul(x14._super, 536870908u), x43);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:46)
let x47: Val = add(mul(x44, 1073741816u), x46);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:47)
let x48: Val = add(mul(x43, 1073741816u), x45);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:48)
let x49: Val = add(x46, x48);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:49)
let x50: Val = add(x45, x47);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:42)
let x51: Val = add(x15._super, x16._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:43)
let x52: Val = add(x17._super, x18._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:44)
let x53: Val = add(mul(x16._super, 536870908u), x52);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:45)
let x54: Val = add(mul(x18._super, 536870908u), x51);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:46)
let x55: Val = add(mul(x52, 1073741816u), x54);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:47)
let x56: Val = add(mul(x51, 1073741816u), x53);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:48)
let x57: Val = add(x54, x56);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:49)
let x58: Val = add(x53, x55);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:42)
let x59: Val = add(x19._super, x20._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:43)
let x60: Val = add(x21._super, x22._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:44)
let x61: Val = add(mul(x20._super, 536870908u), x60);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:45)
let x62: Val = add(mul(x22._super, 536870908u), x59);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:46)
let x63: Val = add(mul(x60, 1073741816u), x62);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:47)
let x64: Val = add(mul(x59, 1073741816u), x61);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:48)
let x65: Val = add(x62, x64);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:49)
let x66: Val = add(x61, x63);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:42)
let x67: Val = add(x23._super, x24._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:43)
let x68: Val = add(x25._super, x26._super);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:44)
let x69: Val = add(mul(x24._super, 536870908u), x68);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:45)
let x70: Val = add(mul(x26._super, 536870908u), x67);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:46)
let x71: Val = add(mul(x68, 1073741816u), x70);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:47)
let x72: Val = add(mul(x67, 1073741816u), x69);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:48)
let x73: Val = add(x70, x72);
// MultiplyByCirculant(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:49)
let x74: Val = add(x69, x71);
// ReduceVec4(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:54)
// MultiplyByMExt(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:64)
let x75: Val = add(add(add(x33, x41), x49), x57);
let x76: Val = add(add(add(x32, x40), x48), x56);
let x77: Val = add(add(add(x34, x42), x50), x58);
let x78: Val = add(add(add(x31, x39), x47), x55);
let x79: Val = add(add(x75, x65), x73);
let x80: Val = add(add(x76, x64), x72);
let x81: Val = add(add(x77, x66), x74);
let x82: Val = add(add(x78, x63), x71);
// MultiplyByMExt(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:65)
let x83: MultiplyByMExt_Super_SuperStruct24Array = MultiplyByMExt_Super_SuperStruct24Array(MultiplyByMExt_Super_SuperStruct(add(x33, x79)), MultiplyByMExt_Super_SuperStruct(add(x32, x80)), MultiplyByMExt_Super_SuperStruct(add(x34, x81)), MultiplyByMExt_Super_SuperStruct(add(x31, x82)), MultiplyByMExt_Super_SuperStruct(add(x41, x79)), MultiplyByMExt_Super_SuperStruct(add(x40, x80)), MultiplyByMExt_Super_SuperStruct(add(x42, x81)), MultiplyByMExt_Super_SuperStruct(add(x39, x82)), MultiplyByMExt_Super_SuperStruct(add(x49, x79)), MultiplyByMExt_Super_SuperStruct(add(x48, x80)), MultiplyByMExt_Super_SuperStruct(add(x50, x81)), MultiplyByMExt_Super_SuperStruct(add(x47, x82)), MultiplyByMExt_Super_SuperStruct(add(x57, x79)), MultiplyByMExt_Super_SuperStruct(add(x56, x80)), MultiplyByMExt_Super_SuperStruct(add(x58, x81)), MultiplyByMExt_Super_SuperStruct(add(x55, x82)), MultiplyByMExt_Super_SuperStruct(add(x65, x79)), MultiplyByMExt_Super_SuperStruct(add(x64, x80)), MultiplyByMExt_Super_SuperStruct(add(x66, x81)), MultiplyByMExt_Super_SuperStruct(add(x63, x82)), MultiplyByMExt_Super_SuperStruct(add(x73, x79)), MultiplyByMExt_Super_SuperStruct(add(x72, x80)), MultiplyByMExt_Super_SuperStruct(add(x74, x81)), MultiplyByMExt_Super_SuperStruct(add(x71, x82)));
return MultiplyByMExtStruct(x83);
}
fn exec_DoExtRoundByIdx(arg0: Val24Array, arg1_0: Val, layout2: BoundLayout_DoExtRoundByIdxLayout) -> MultiplyByMExtStruct {
// DoExtRoundByIdx(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:115)
let x3: OneHot_8_Struct = exec_OneHot_8_(arg1_0, lookup_DoExtRoundByIdxLayout_idxHot(layout2));
// builtin Mul
// MultBy(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:111)
// DoExtRoundByIdx(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:119)
let x4: Val = mul(x3._super[decode(0u)]._super, 514215000u);
let x5: Val = mul(x3._super[decode(0u)]._super, 1473356547u);
let x6: Val = mul(x3._super[decode(0u)]._super, 1475549808u);
let x7: Val = mul(x3._super[decode(0u)]._super, 280201050u);
let x8: Val = mul(x3._super[decode(0u)]._super, 66476557u);
let x9: Val = mul(x3._super[decode(0u)]._super, 553479290u);
let x10: Val = mul(x3._super[decode(0u)]._super, 103276920u);
let x11: Val = mul(x3._super[decode(0u)]._super, 1615072023u);
let x12: Val = mul(x3._super[decode(0u)]._super, 810837962u);
let x13: Val = mul(x3._super[decode(0u)]._super, 646554063u);
let x14: Val = mul(x3._super[decode(0u)]._super, 1079828279u);
let x15: Val = mul(x3._super[decode(0u)]._super, 1652363045u);
let x16: Val = mul(x3._super[decode(0u)]._super, 569019161u);
let x17: Val = mul(x3._super[decode(0u)]._super, 901639906u);
let x18: Val = mul(x3._super[decode(0u)]._super, 1154652333u);
let x19: Val = mul(x3._super[decode(0u)]._super, 1197195121u);
let x20: Val = mul(x3._super[decode(0u)]._super, 1146559378u);
let x21: Val = mul(x3._super[decode(0u)]._super, 1908415181u);
let x22: Val = mul(x3._super[decode(0u)]._super, 1188553533u);
let x23: Val = mul(x3._super[decode(0u)]._super, 892287745u);
let x24: Val = mul(x3._super[decode(0u)]._super, 1849641539u);
let x25: Val = mul(x3._super[decode(0u)]._super, 1631541944u);
let x26: Val = mul(x3._super[decode(0u)]._super, 725508932u);
let x27: Val = mul(x3._super[decode(0u)]._super, 104512978u);
let x28: Val = mul(x3._super[decode(268435454u)]._super, 1334122235u);
let x29: Val = mul(x3._super[decode(268435454u)]._super, 491896527u);
let x30: Val = mul(x3._super[decode(268435454u)]._super, 54459111u);
let x31: Val = mul(x3._super[decode(268435454u)]._super, 376068648u);
let x32: Val = mul(x3._super[decode(268435454u)]._super, 203528896u);
let x33: Val = mul(x3._super[decode(268435454u)]._super, 54074077u);
let x34: Val = mul(x3._super[decode(268435454u)]._super, 1660407700u);
let x35: Val = mul(x3._super[decode(268435454u)]._super, 518066980u);
let x36: Val = mul(x3._super[decode(268435454u)]._super, 1973611996u);
let x37: Val = mul(x3._super[decode(268435454u)]._super, 282379828u);
let x38: Val = mul(x3._super[decode(268435454u)]._super, 1621744025u);
let x39: Val = mul(x3._super[decode(268435454u)]._super, 1797291911u);
let x40: Val = mul(x3._super[decode(268435454u)]._super, 1693741551u);
let x41: Val = mul(x3._super[decode(268435454u)]._super, 1978660371u);
let x42: Val = mul(x3._super[decode(268435454u)]._super, 886544622u);
let x43: Val = mul(x3._super[decode(268435454u)]._super, 167843215u);
let x44: Val = mul(x3._super[decode(268435454u)]._super, 766618548u);
let x45: Val = mul(x3._super[decode(268435454u)]._super, 1464263490u);
let x46: Val = mul(x3._super[decode(268435454u)]._super, 1143720041u);
let x47: Val = mul(x3._super[decode(268435454u)]._super, 1571250914u);
let x48: Val = mul(x3._super[decode(268435454u)]._super, 1097878221u);
let x49: Val = mul(x3._super[decode(268435454u)]._super, 138511063u);
let x50: Val = mul(x3._super[decode(268435454u)]._super, 1364260882u);
let x51: Val = mul(x3._super[decode(268435454u)]._super, 1916288354u);
let x52: Val = mul(x3._super[decode(536870908u)]._super, 773178408u);
let x53: Val = mul(x3._super[decode(536870908u)]._super, 1821171954u);
let x54: Val = mul(x3._super[decode(536870908u)]._super, 403657849u);
let x55: Val = mul(x3._super[decode(536870908u)]._super, 1575896402u);
let x56: Val = mul(x3._super[decode(536870908u)]._super, 73837948u);
let x57: Val = mul(x3._super[decode(536870908u)]._super, 743839936u);
let x58: Val = mul(x3._super[decode(536870908u)]._super, 591330190u);
let x59: Val = mul(x3._super[decode(536870908u)]._super, 1433978562u);
let x60: Val = mul(x3._super[decode(536870908u)]._super, 262499807u);
let x61: Val = mul(x3._super[decode(536870908u)]._super, 431526004u);
let x62: Val = mul(x3._super[decode(536870908u)]._super, 719595300u);
let x63: Val = mul(x3._super[decode(536870908u)]._super, 194223030u);
let x64: Val = mul(x3._super[decode(536870908u)]._super, 1265398957u);
let x65: Val = mul(x3._super[decode(536870908u)]._super, 1935012223u);
let x66: Val = mul(x3._super[decode(536870908u)]._super, 1757830457u);
let x67: Val = mul(x3._super[decode(536870908u)]._super, 856083153u);
let x68: Val = mul(x3._super[decode(536870908u)]._super, 1565856438u);
let x69: Val = mul(x3._super[decode(536870908u)]._super, 833999680u);
let x70: Val = mul(x3._super[decode(536870908u)]._super, 113987054u);
let x71: Val = mul(x3._super[decode(536870908u)]._super, 1855218953u);
let x72: Val = mul(x3._super[decode(536870908u)]._super, 1681459362u);
let x73: Val = mul(x3._super[decode(536870908u)]._super, 890486603u);
let x74: Val = mul(x3._super[decode(536870908u)]._super, 521608901u);
let x75: Val = mul(x3._super[decode(536870908u)]._super, 173274721u);
let x76: Val = mul(x3._super[decode(805306362u)]._super, 1470741859u);
let x77: Val = mul(x3._super[decode(805306362u)]._super, 504959283u);
let x78: Val = mul(x3._super[decode(805306362u)]._super, 1923302440u);
let x79: Val = mul(x3._super[decode(805306362u)]._super, 334947417u);
let x80: Val = mul(x3._super[decode(805306362u)]._super, 1416053552u);
let x81: Val = mul(x3._super[decode(805306362u)]._super, 118182962u);
let x82: Val = mul(x3._super[decode(805306362u)]._super, 1100773057u);
let x83: Val = mul(x3._super[decode(805306362u)]._super, 225906547u);
let x84: Val = mul(x3._super[decode(805306362u)]._super, 1696758794u);
let x85: Val = mul(x3._super[decode(805306362u)]._super, 235400587u);
let x86: Val = mul(x3._super[decode(805306362u)]._super, 1997205562u);
let x87: Val = mul(x3._super[decode(805306362u)]._super, 716176367u);
let x88: Val = mul(x3._super[decode(805306362u)]._super, 499124855u);
let x89: Val = mul(x3._super[decode(805306362u)]._super, 1720932111u);
let x90: Val = mul(x3._super[decode(805306362u)]._super, 289571341u);
let x91: Val = mul(x3._super[decode(805306362u)]._super, 1719460361u);
let x92: Val = mul(x3._super[decode(805306362u)]._super, 1762284302u);
let x93: Val = mul(x3._super[decode(805306362u)]._super, 1510804498u);
let x94: Val = mul(x3._super[decode(805306362u)]._super, 1501266228u);
let x95: Val = mul(x3._super[decode(805306362u)]._super, 1958052382u);
let x96: Val = mul(x3._super[decode(805306362u)]._super, 1650199808u);
let x97: Val = mul(x3._super[decode(805306362u)]._super, 355437897u);
let x98: Val = mul(x3._super[decode(805306362u)]._super, 1969804773u);
let x99: Val = mul(x3._super[decode(805306362u)]._super, 1941801293u);
let x100: Val = mul(x3._super[decode(1073741816u)]._super, 1497457364u);
let x101: Val = mul(x3._super[decode(1073741816u)]._super, 1276091099u);
let x102: Val = mul(x3._super[decode(1073741816u)]._super, 1894167051u);
let x103: Val = mul(x3._super[decode(1073741816u)]._super, 1333627684u);
let x104: Val = mul(x3._super[decode(1073741816u)]._super, 1171979540u);
let x105: Val = mul(x3._super[decode(1073741816u)]._super, 1349891480u);
let x106: Val = mul(x3._super[decode(1073741816u)]._super, 1848354674u);
let x107: Val = mul(x3._super[decode(1073741816u)]._super, 744154815u);
let x108: Val = mul(x3._super[decode(1073741816u)]._super, 1950007540u);
let x109: Val = mul(x3._super[decode(1073741816u)]._super, 819092480u);
let x110: Val = mul(x3._super[decode(1073741816u)]._super, 1956018105u);
let x111: Val = mul(x3._super[decode(1073741816u)]._super, 1115373549u);
let x112: Val = mul(x3._super[decode(1073741816u)]._super, 1122988591u);
let x113: Val = mul(x3._super[decode(1073741816u)]._super, 433670620u);
let x114: Val = mul(x3._super[decode(1073741816u)]._super, 1004751065u);
let x115: Val = mul(x3._super[decode(1073741816u)]._super, 486579077u);
let x116: Val = mul(x3._super[decode(1073741816u)]._super, 1352768232u);
let x117: Val = mul(x3._super[decode(1073741816u)]._super, 947535130u);
let x118: Val = mul(x3._super[decode(1073741816u)]._super, 684578900u);
let x119: Val = mul(x3._super[decode(1073741816u)]._super, 556404990u);
let x120: Val = mul(x3._super[decode(1073741816u)]._super, 177487993u);
let x121: Val = mul(x3._super[decode(1073741816u)]._super, 486074613u);
let x122: Val = mul(x3._super[decode(1073741816u)]._super, 1813250422u);
let x123: Val = mul(x3._super[decode(1073741816u)]._super, 732038186u);
let x124: Val = mul(x3._super[decode(1342177270u)]._super, 641675496u);
let x125: Val = mul(x3._super[decode(1342177270u)]._super, 85370036u);
let x126: Val = mul(x3._super[decode(1342177270u)]._super, 1046322034u);
let x127: Val = mul(x3._super[decode(1342177270u)]._super, 1839987247u);
let x128: Val = mul(x3._super[decode(1342177270u)]._super, 326383233u);
let x129: Val = mul(x3._super[decode(1342177270u)]._super, 1360384116u);
let x130: Val = mul(x3._super[decode(1342177270u)]._super, 1042184086u);
let x131: Val = mul(x3._super[decode(1342177270u)]._super, 833417546u);
let x132: Val = mul(x3._super[decode(1342177270u)]._super, 129595121u);
let x133: Val = mul(x3._super[decode(1342177270u)]._super, 1442408468u);
let x134: Val = mul(x3._super[decode(1342177270u)]._super, 979417802u);
let x135: Val = mul(x3._super[decode(1342177270u)]._super, 1710561480u);
let x136: Val = mul(x3._super[decode(1342177270u)]._super, 1048383294u);
let x137: Val = mul(x3._super[decode(1342177270u)]._super, 1311937500u);
let x138: Val = mul(x3._super[decode(1342177270u)]._super, 808034843u);
let x139: Val = mul(x3._super[decode(1342177270u)]._super, 269284270u);
let x140: Val = mul(x3._super[decode(1342177270u)]._super, 1504218145u);
let x141: Val = mul(x3._super[decode(1342177270u)]._super, 297623519u);
let x142: Val = mul(x3._super[decode(1342177270u)]._super, 1574558398u);
let x143: Val = mul(x3._super[decode(1342177270u)]._super, 1512692245u);
let x144: Val = mul(x3._super[decode(1342177270u)]._super, 1132490863u);
let x145: Val = mul(x3._super[decode(1342177270u)]._super, 634765248u);
let x146: Val = mul(x3._super[decode(1342177270u)]._super, 803776238u);
let x147: Val = mul(x3._super[decode(1342177270u)]._super, 484649754u);
let x148: Val = mul(x3._super[decode(1610612724u)]._super, 682278882u);
let x149: Val = mul(x3._super[decode(1610612724u)]._super, 1014915678u);
let x150: Val = mul(x3._super[decode(1610612724u)]._super, 1476229142u);
let x151: Val = mul(x3._super[decode(1610612724u)]._super, 2004304323u);
let x152: Val = mul(x3._super[decode(1610612724u)]._super, 529748649u);
let x153: Val = mul(x3._super[decode(1610612724u)]._super, 1721394210u);
let x154: Val = mul(x3._super[decode(1610612724u)]._super, 1441645470u);
let x155: Val = mul(x3._super[decode(1610612724u)]._super, 1858910116u);
let x156: Val = mul(x3._super[decode(1610612724u)]._super, 198424796u);
let x157: Val = mul(x3._super[decode(1610612724u)]._super, 1436920111u);
let x158: Val = mul(x3._super[decode(1610612724u)]._super, 1215222582u);
let x159: Val = mul(x3._super[decode(1610612724u)]._super, 933370502u);
let x160: Val = mul(x3._super[decode(1610612724u)]._super, 143759022u);
let x161: Val = mul(x3._super[decode(1610612724u)]._super, 1035452870u);
let x162: Val = mul(x3._super[decode(1610612724u)]._super, 460823304u);
let x163: Val = mul(x3._super[decode(1610612724u)]._super, 909965784u);
let x164: Val = mul(x3._super[decode(1610612724u)]._super, 201144279u);
let x165: Val = mul(x3._super[decode(1610612724u)]._super, 429997713u);
let x166: Val = mul(x3._super[decode(1610612724u)]._super, 634885780u);
let x167: Val = mul(x3._super[decode(1610612724u)]._super, 1036892459u);
let x168: Val = mul(x3._super[decode(1610612724u)]._super, 257468050u);
let x169: Val = mul(x3._super[decode(1610612724u)]._super, 325299453u);
let x170: Val = mul(x3._super[decode(1610612724u)]._super, 1481301005u);
let x171: Val = mul(x3._super[decode(1610612724u)]._super, 127711554u);
let x172: Val = mul(x3._super[decode(1879048178u)]._super, 377983626u);
let x173: Val = mul(x3._super[decode(1879048178u)]._super, 1860200820u);
let x174: Val = mul(x3._super[decode(1879048178u)]._super, 1750890547u);
let x175: Val = mul(x3._super[decode(1879048178u)]._super, 304452178u);
let x176: Val = mul(x3._super[decode(1879048178u)]._super, 152103645u);
let x177: Val = mul(x3._super[decode(1879048178u)]._super, 801274586u);
let x178: Val = mul(x3._super[decode(1879048178u)]._super, 1888040085u);
let x179: Val = mul(x3._super[decode(1879048178u)]._super, 1659972272u);
let x180: Val = mul(x3._super[decode(1879048178u)]._super, 787408365u);
let x181: Val = mul(x3._super[decode(1879048178u)]._super, 883701053u);
let x182: Val = mul(x3._super[decode(1879048178u)]._super, 763866915u);
let x183: Val = mul(x3._super[decode(1879048178u)]._super, 1581752865u);
let x184: Val = mul(x3._super[decode(1879048178u)]._super, 1932512397u);
let x185: Val = mul(x3._super[decode(1879048178u)]._super, 1119094246u);
let x186: Val = mul(x3._super[decode(1879048178u)]._super, 625785354u);
let x187: Val = mul(x3._super[decode(1879048178u)]._super, 912815711u);
let x188: Val = mul(x3._super[decode(1879048178u)]._super, 1053366786u);
let x189: Val = mul(x3._super[decode(1879048178u)]._super, 1246219010u);
let x190: Val = mul(x3._super[decode(1879048178u)]._super, 17879907u);
let x191: Val = mul(x3._super[decode(1879048178u)]._super, 1935021695u);
let x192: Val = mul(x3._super[decode(1879048178u)]._super, 1467816753u);
let x193: Val = mul(x3._super[decode(1879048178u)]._super, 324912627u);
let x194: Val = mul(x3._super[decode(1879048178u)]._super, 191577525u);
let x195: Val = mul(x3._super[decode(1879048178u)]._super, 1753091373u);
// builtin Add
// AddConsts(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:107)
// DoExtRoundByIdx(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:118)
let x196: Val = add(add(add(x4, x28), x52), x76);
let x197: Val = add(add(add(x5, x29), x53), x77);
let x198: Val = add(add(add(x6, x30), x54), x78);
let x199: Val = add(add(add(x7, x31), x55), x79);
let x200: Val = add(add(add(x8, x32), x56), x80);
let x201: Val = add(add(add(x9, x33), x57), x81);
let x202: Val = add(add(add(x10, x34), x58), x82);
let x203: Val = add(add(add(x11, x35), x59), x83);
let x204: Val = add(add(add(x12, x36), x60), x84);
let x205: Val = add(add(add(x13, x37), x61), x85);
let x206: Val = add(add(add(x14, x38), x62), x86);
let x207: Val = add(add(add(x15, x39), x63), x87);
let x208: Val = add(add(add(x16, x40), x64), x88);
let x209: Val = add(add(add(x17, x41), x65), x89);
let x210: Val = add(add(add(x18, x42), x66), x90);
let x211: Val = add(add(add(x19, x43), x67), x91);
let x212: Val = add(add(add(x20, x44), x68), x92);
let x213: Val = add(add(add(x21, x45), x69), x93);
let x214: Val = add(add(add(x22, x46), x70), x94);
let x215: Val = add(add(add(x23, x47), x71), x95);
let x216: Val = add(add(add(x24, x48), x72), x96);
let x217: Val = add(add(add(x25, x49), x73), x97);
let x218: Val = add(add(add(x26, x50), x74), x98);
let x219: Val = add(add(add(x27, x51), x75), x99);
let x220: Val = add(add(add(x196, x100), x124), x148);
let x221: Val = add(add(add(x197, x101), x125), x149);
let x222: Val = add(add(add(x198, x102), x126), x150);
let x223: Val = add(add(add(x199, x103), x127), x151);
let x224: Val = add(add(add(x200, x104), x128), x152);
let x225: Val = add(add(add(x201, x105), x129), x153);
let x226: Val = add(add(add(x202, x106), x130), x154);
let x227: Val = add(add(add(x203, x107), x131), x155);
let x228: Val = add(add(add(x204, x108), x132), x156);
let x229: Val = add(add(add(x205, x109), x133), x157);
let x230: Val = add(add(add(x206, x110), x134), x158);
let x231: Val = add(add(add(x207, x111), x135), x159);
let x232: Val = add(add(add(x208, x112), x136), x160);
let x233: Val = add(add(add(x209, x113), x137), x161);
let x234: Val = add(add(add(x210, x114), x138), x162);
let x235: Val = add(add(add(x211, x115), x139), x163);
let x236: Val = add(add(add(x212, x116), x140), x164);
let x237: Val = add(add(add(x213, x117), x141), x165);
let x238: Val = add(add(add(x214, x118), x142), x166);
let x239: Val = add(add(add(x215, x119), x143), x167);
let x240: Val = add(add(add(x216, x120), x144), x168);
let x241: Val = add(add(add(x217, x121), x145), x169);
let x242: Val = add(add(add(x218, x122), x146), x170);
let x243: Val = add(add(add(x219, x123), x147), x171);
// DoExtRoundByIdx(zirgen/circuit/rv32im/v2/dsl/poseidon2.zir:122)
let x244: MultiplyByMExtStruct = exec_DoExtRound(arg0, Val24Array(add(x220, x172), add(x221, x173), add(x222, x174), add(x223, x175), add(x224, x176), add(x225, x177), add(x226, x178), add(x227, x179), add(x228, x180), add(x229, x181), add(x230, x182), add(x231, x183), add(x232, x184), add(x233, x185), add(x234, x186), add(x235, x187), add(x236, x188), add(x237, x189), add(x238, x190), add(x239, x191), add(x240, x192), add(x241, x193), add(x242, x194), add(x243, x195)), lookup_DoExtRoundByIdxLayout__super(layout2));
return x244;
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
fn exec_PoseidonExtRound(arg0: PoseidonStateStruct, layout1: BoundLayout_PoseidonExtRoundLayout) -> PoseidonStateStruct {
// builtin Sub
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:241)
let x2: Val = sub(arg0.subState._super, 805306362u);
let x3: NondetRegStruct = exec_IsZero(x2, lookup_PoseidonExtRoundLayout_isRound3(layout1));
// builtin Sub
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:242)
let x4: Val = sub(arg0.subState._super, 1879048178u);
let x5: NondetRegStruct = exec_IsZero(x4, lookup_PoseidonExtRoundLayout_isRound7(layout1));
// builtin Sub
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:243)
let x6: Val = sub(arg0.count._super, 268435454u);
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:244)
let x7: NondetRegStruct = exec_IsZero(x6, lookup_PoseidonExtRoundLayout_lastBlock(layout1));
// builtin Sub
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:245)
let x8: Val = sub(arg0.count._super, x5._super);
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:248)
let x9: Val = sub(sub(268435454u, x3._super), x5._super);
// builtin Add
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:247)
let x10: Val = add(mul(x3._super, 671088587u), mul(x9, 402653133u));
// builtin Mul
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:249)
let x11: Val = mul(x5._super, sub(268435454u, x7._super));
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:250)
let x12: Val = mul(mul(x5._super, x7._super), 1610612692u);
// builtin Add
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:251)
let x13: Val = add(arg0.subState._super, 268435454u);
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:240)
let x14: Val24Array = Val24Array(arg0.inner[decode(0u)]._super, arg0.inner[decode(268435454u)]._super, arg0.inner[decode(536870908u)]._super, arg0.inner[decode(805306362u)]._super, arg0.inner[decode(1073741816u)]._super, arg0.inner[decode(1342177270u)]._super, arg0.inner[decode(1610612724u)]._super, arg0.inner[decode(1879048178u)]._super, arg0.inner[decode(134217711u)]._super, arg0.inner[decode(402653165u)]._super, arg0.inner[decode(671088619u)]._super, arg0.inner[decode(939524073u)]._super, arg0.inner[decode(1207959527u)]._super, arg0.inner[decode(1476394981u)]._super, arg0.inner[decode(1744830435u)]._super, arg0.inner[decode(2013265889u)]._super, arg0.inner[decode(268435422u)]._super, arg0.inner[decode(536870876u)]._super, arg0.inner[decode(805306330u)]._super, arg0.inner[decode(1073741784u)]._super, arg0.inner[decode(1342177238u)]._super, arg0.inner[decode(1610612692u)]._super, arg0.inner[decode(1879048146u)]._super, arg0.inner[decode(134217679u)]._super);
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:252)
let x15: MultiplyByMExtStruct = exec_DoExtRoundByIdx(x14, arg0.subState._super, lookup_PoseidonExtRoundLayout_nextInner(layout1));
// PoseidonOpDef(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:8)
// GetDef(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:72)
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:253)
let x16: PoseidonOpDefStruct = PoseidonOpDefStruct(arg0.hasState._super, arg0.stateAddr._super, arg0.bufOutAddr._super, arg0.isElem._super, arg0.checkOut._super, arg0.loadTxType._super);
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:252)
let x17: Val24Array = Val24Array(x15._super[decode(0u)]._super, x15._super[decode(268435454u)]._super, x15._super[decode(536870908u)]._super, x15._super[decode(805306362u)]._super, x15._super[decode(1073741816u)]._super, x15._super[decode(1342177270u)]._super, x15._super[decode(1610612724u)]._super, x15._super[decode(1879048178u)]._super, x15._super[decode(134217711u)]._super, x15._super[decode(402653165u)]._super, x15._super[decode(671088619u)]._super, x15._super[decode(939524073u)]._super, x15._super[decode(1207959527u)]._super, x15._super[decode(1476394981u)]._super, x15._super[decode(1744830435u)]._super, x15._super[decode(2013265889u)]._super, x15._super[decode(268435422u)]._super, x15._super[decode(536870876u)]._super, x15._super[decode(805306330u)]._super, x15._super[decode(1073741784u)]._super, x15._super[decode(1342177238u)]._super, x15._super[decode(1610612692u)]._super, x15._super[decode(1879048146u)]._super, x15._super[decode(134217679u)]._super);
// PoseidonExtRound(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:253)
let x18: PoseidonStateStruct = exec_PoseidonState(x16, add(add(x10, mul(x11, 805306330u)), x12), mul(x9, x13), arg0.bufInAddr._super, x8, arg0.mode._super, x17, arg0.zcheck._super, lookup_PoseidonExtRoundLayout__super(layout1));
return x18;
}
fn exec_Poseidon1Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Poseidon1Layout) -> InstOutputBaseStruct {
// Poseidon1(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:486)
let x3: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Poseidon1Layout__0(layout2));
// Poseidon1(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:487)
let x4: Val = sub(arg1_0.state, add(arg1_0.minor, 402653133u));
eqz(x4);
var x5: PoseidonStateStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Poseidon1(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:490)
let x6: PoseidonStateStruct = back_PoseidonState(1, lookup_Poseidon1Layout_state(layout2));
let x7: PoseidonStateStruct = exec_PoseidonExtRound(x6, lookup_Poseidon1StateLayout_arm0(lookup_Poseidon1Layout_stateRedef(layout2)));
x5 = x7;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x8: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Poseidon1(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:489)
let x9: PoseidonStateStruct = back_PoseidonState(0, lookup_Poseidon1StateLayout__super(lookup_Poseidon1Layout_stateRedef(layout2)));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// Poseidon1(zirgen/circuit/rv32im/v2/dsl/inst_p2.zir:499)
let x10: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
// InstOutputBase(zirgen/circuit/rv32im/v2/dsl/inst.zir:78)
let x11: InstOutputBaseStruct = InstOutputBaseStruct(arg1_0.pcU32, x9.nextState._super, x9.mode._super, x10);
x8 = x11;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x8;
}
