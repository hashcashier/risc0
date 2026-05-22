// Generated SP7 TopAccum arm 5 browser capacity probe.

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

fn back_BigIntAccumState(distance0: Index, layout1: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// BigIntAccumState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:189)
let x2: NondetExtRegStruct = back_ExtReg(distance0, lookup_BigIntAccumStateLayout_poly(layout1));
// BigIntAccumState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:190)
let x3: NondetExtRegStruct = back_ExtReg(distance0, lookup_BigIntAccumStateLayout_term(layout1));
// BigIntAccumState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:191)
let x4: NondetExtRegStruct = back_ExtReg(distance0, lookup_BigIntAccumStateLayout_total(layout1));
return BigIntAccumStateStruct(x2, x3, x4);
}

fn exec_BigIntAccumState(arg0: ExtVal, arg1_0: ExtVal, arg2_0: ExtVal, layout3: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// BigIntAccumState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:189)
let x4: NondetExtRegStruct = exec_ExtReg(arg0, lookup_BigIntAccumStateLayout_poly(layout3));
// BigIntAccumState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:190)
let x5: NondetExtRegStruct = exec_ExtReg(arg1_0, lookup_BigIntAccumStateLayout_term(layout3));
// BigIntAccumState(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:191)
let x6: NondetExtRegStruct = exec_ExtReg(arg2_0, lookup_BigIntAccumStateLayout_total(layout3));
return BigIntAccumStateStruct(x4, x5, x6);
}

fn exec_OneHot_7_(arg0: Val, layout1: BoundLayout_OneHot_7_Layout) -> OneHot_7_Struct {
// builtin Isz
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:7)
let x2: Val = isz(sub(0u, arg0));
let x3: NondetRegStruct = exec_NondetBitReg(x2, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(0u)));
// builtin Isz
let x4: Val = isz(sub(268435454u, arg0));
let x5: NondetRegStruct = exec_NondetBitReg(x4, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(268435454u)));
// builtin Isz
let x6: Val = isz(sub(536870908u, arg0));
let x7: NondetRegStruct = exec_NondetBitReg(x6, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(536870908u)));
// builtin Isz
let x8: Val = isz(sub(805306362u, arg0));
let x9: NondetRegStruct = exec_NondetBitReg(x8, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(805306362u)));
// builtin Isz
let x10: Val = isz(sub(1073741816u, arg0));
let x11: NondetRegStruct = exec_NondetBitReg(x10, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(1073741816u)));
// builtin Isz
let x12: Val = isz(sub(1342177270u, arg0));
let x13: NondetRegStruct = exec_NondetBitReg(x12, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(1342177270u)));
// builtin Isz
let x14: Val = isz(sub(1610612724u, arg0));
let x15: NondetRegStruct = exec_NondetBitReg(x14, subscript_NondetRegLayout7LayoutArray(lookup_OneHot_7_Layout__super(layout1), decode(1610612724u)));
// builtin Add
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:9)
let x16: Val = add(add(x3._super, x5._super), x7._super);
let x17: Val = add(add(add(x16, x9._super), x11._super), x13._super);
eqz(sub(add(x17, x15._super), 268435454u));
// builtin Add
// OneHot(zirgen/circuit/rv32im/v2/dsl/one_hot.zir:11)
let x18: Val = add(x5._super, mul(x7._super, 536870908u));
let x19: Val = add(add(x18, mul(x9._super, 805306362u)), mul(x11._super, 1073741816u));
let x20: Val = add(add(x19, mul(x13._super, 1342177270u)), mul(x15._super, 1610612724u));
eqz(sub(x20, arg0));
return OneHot_7_Struct(NondetRegStruct7Array(x3, x5, x7, x9, x11, x13, x15));
}

fn exec_BigIntPolyOpNop(layout0: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// BigIntPolyOpNop(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:251)
let x1: BigIntAccumStateStruct = exec_BigIntAccumState(ExtVal(0u, 0u, 0u, 0u), ExtVal(268435454u, 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u), layout0);
return x1;
}

fn exec_BigIntPolyOpShift(arg0: BigIntTopStateStruct, arg1_0: BigIntAccumStateStruct, arg2_0: ExtVal, layout3: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// builtin ExtMul
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:210)
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:236)
// BigIntPolyOpShift(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:260)
let x4: ExtVal = ext_mul(arg2_0, ExtVal(268435454u, 0u, 0u, 0u));
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:211)
let x5: ExtVal = ext_mul(x4, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:212)
let x6: ExtVal = ext_mul(x5, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:213)
let x7: ExtVal = ext_mul(x6, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:214)
let x8: ExtVal = ext_mul(x7, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:215)
let x9: ExtVal = ext_mul(x8, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:216)
let x10: ExtVal = ext_mul(x9, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:217)
let x11: ExtVal = ext_mul(x10, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:218)
let x12: ExtVal = ext_mul(x11, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:219)
let x13: ExtVal = ext_mul(x12, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:220)
let x14: ExtVal = ext_mul(x13, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:221)
let x15: ExtVal = ext_mul(x14, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:222)
let x16: ExtVal = ext_mul(x15, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:223)
let x17: ExtVal = ext_mul(x16, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:224)
let x18: ExtVal = ext_mul(x17, arg2_0);
// builtin MakeExt
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:239)
let x19: ExtVal = ext_add(ExtVal(arg0.witness[decode(0u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x20: ExtVal = ext_add(ExtVal(arg0.witness[decode(268435454u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x21: ExtVal = ext_add(ExtVal(arg0.witness[decode(536870908u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x22: ExtVal = ext_add(ExtVal(arg0.witness[decode(805306362u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x23: ExtVal = ext_add(ExtVal(arg0.witness[decode(1073741816u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x24: ExtVal = ext_add(ExtVal(arg0.witness[decode(1342177270u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x25: ExtVal = ext_add(ExtVal(arg0.witness[decode(1610612724u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x26: ExtVal = ext_add(ExtVal(arg0.witness[decode(1879048178u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x27: ExtVal = ext_add(ExtVal(arg0.witness[decode(134217711u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x28: ExtVal = ext_add(ExtVal(arg0.witness[decode(402653165u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x29: ExtVal = ext_add(ExtVal(arg0.witness[decode(671088619u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x30: ExtVal = ext_add(ExtVal(arg0.witness[decode(939524073u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x31: ExtVal = ext_add(ExtVal(arg0.witness[decode(1207959527u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x32: ExtVal = ext_add(ExtVal(arg0.witness[decode(1476394981u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x33: ExtVal = ext_add(ExtVal(arg0.witness[decode(1744830435u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x34: ExtVal = ext_add(ExtVal(arg0.witness[decode(2013265889u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtAdd
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:238)
let x35: ExtVal = ext_add(ext_add(ext_mul(x19, ExtVal(268435454u, 0u, 0u, 0u)), ExtVal(0u, 0u, 0u, 0u)), ext_mul(x4, x20));
let x36: ExtVal = ext_add(ext_add(ext_add(x35, ext_mul(x5, x21)), ext_mul(x6, x22)), ext_mul(x7, x23));
let x37: ExtVal = ext_add(ext_add(ext_add(x36, ext_mul(x8, x24)), ext_mul(x9, x25)), ext_mul(x10, x26));
let x38: ExtVal = ext_add(ext_add(ext_add(x37, ext_mul(x11, x27)), ext_mul(x12, x28)), ext_mul(x13, x29));
let x39: ExtVal = ext_add(ext_add(ext_add(x38, ext_mul(x14, x30)), ext_mul(x15, x31)), ext_mul(x16, x32));
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:246)
let x40: ExtVal = ext_add(arg1_0.poly._super, ext_add(ext_add(x39, ext_mul(x17, x33)), ext_mul(x18, x34)));
// BigIntPolyOpShift(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:261)
let x41: BigIntAccumStateStruct = exec_BigIntAccumState(ext_mul(x40, ext_mul(x18, arg2_0)), arg1_0.term._super, arg1_0.total._super, layout3);
return x41;
}

fn exec_BigIntPolyOpSetTerm(arg0: BigIntTopStateStruct, arg1_0: BigIntAccumStateStruct, arg2_0: ExtVal, layout3: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// builtin ExtMul
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:210)
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:236)
// BigIntPolyOpSetTerm(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:270)
let x4: ExtVal = ext_mul(arg2_0, ExtVal(268435454u, 0u, 0u, 0u));
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:211)
let x5: ExtVal = ext_mul(x4, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:212)
let x6: ExtVal = ext_mul(x5, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:213)
let x7: ExtVal = ext_mul(x6, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:214)
let x8: ExtVal = ext_mul(x7, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:215)
let x9: ExtVal = ext_mul(x8, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:216)
let x10: ExtVal = ext_mul(x9, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:217)
let x11: ExtVal = ext_mul(x10, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:218)
let x12: ExtVal = ext_mul(x11, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:219)
let x13: ExtVal = ext_mul(x12, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:220)
let x14: ExtVal = ext_mul(x13, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:221)
let x15: ExtVal = ext_mul(x14, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:222)
let x16: ExtVal = ext_mul(x15, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:223)
let x17: ExtVal = ext_mul(x16, arg2_0);
// builtin MakeExt
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:239)
let x18: ExtVal = ext_add(ExtVal(arg0.witness[decode(0u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x19: ExtVal = ext_add(ExtVal(arg0.witness[decode(268435454u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x20: ExtVal = ext_add(ExtVal(arg0.witness[decode(536870908u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x21: ExtVal = ext_add(ExtVal(arg0.witness[decode(805306362u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x22: ExtVal = ext_add(ExtVal(arg0.witness[decode(1073741816u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x23: ExtVal = ext_add(ExtVal(arg0.witness[decode(1342177270u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x24: ExtVal = ext_add(ExtVal(arg0.witness[decode(1610612724u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x25: ExtVal = ext_add(ExtVal(arg0.witness[decode(1879048178u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x26: ExtVal = ext_add(ExtVal(arg0.witness[decode(134217711u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x27: ExtVal = ext_add(ExtVal(arg0.witness[decode(402653165u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x28: ExtVal = ext_add(ExtVal(arg0.witness[decode(671088619u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x29: ExtVal = ext_add(ExtVal(arg0.witness[decode(939524073u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x30: ExtVal = ext_add(ExtVal(arg0.witness[decode(1207959527u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x31: ExtVal = ext_add(ExtVal(arg0.witness[decode(1476394981u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x32: ExtVal = ext_add(ExtVal(arg0.witness[decode(1744830435u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x33: ExtVal = ext_add(ExtVal(arg0.witness[decode(2013265889u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtAdd
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:238)
let x34: ExtVal = ext_add(ext_add(ext_mul(x18, ExtVal(268435454u, 0u, 0u, 0u)), ExtVal(0u, 0u, 0u, 0u)), ext_mul(x4, x19));
let x35: ExtVal = ext_add(ext_add(ext_add(x34, ext_mul(x5, x20)), ext_mul(x6, x21)), ext_mul(x7, x22));
let x36: ExtVal = ext_add(ext_add(ext_add(x35, ext_mul(x8, x23)), ext_mul(x9, x24)), ext_mul(x10, x25));
let x37: ExtVal = ext_add(ext_add(ext_add(x36, ext_mul(x11, x26)), ext_mul(x12, x27)), ext_mul(x13, x28));
let x38: ExtVal = ext_add(ext_add(ext_add(x37, ext_mul(x14, x29)), ext_mul(x15, x30)), ext_mul(x16, x31));
let x39: ExtVal = ext_add(ext_add(x38, ext_mul(x17, x32)), ext_mul(ext_mul(x17, arg2_0), x33));
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:246)
let x40: ExtVal = ext_add(arg1_0.poly._super, x39);
// BigIntPolyOpSetTerm(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:271)
let x41: BigIntAccumStateStruct = exec_BigIntAccumState(ExtVal(0u, 0u, 0u, 0u), x40, arg1_0.total._super, layout3);
return x41;
}

fn exec_BigIntPolyOpAddTotal(arg0: BigIntTopStateStruct, arg1_0: BigIntAccumStateStruct, arg2_0: ExtVal, layout3: BoundLayout_BigIntPolyOpAddTotalLayout) -> BigIntAccumStateStruct {
// builtin MakeExt
// BigIntPolyOpAddTotal(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:280)
let x4: ExtVal = ext_add(ExtVal(sub(arg0.coeff, 1073741816u), 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtMul
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:210)
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:236)
// BigIntPolyOpAddTotal(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:281)
let x5: ExtVal = ext_mul(arg2_0, ExtVal(268435454u, 0u, 0u, 0u));
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:211)
let x6: ExtVal = ext_mul(x5, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:212)
let x7: ExtVal = ext_mul(x6, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:213)
let x8: ExtVal = ext_mul(x7, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:214)
let x9: ExtVal = ext_mul(x8, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:215)
let x10: ExtVal = ext_mul(x9, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:216)
let x11: ExtVal = ext_mul(x10, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:217)
let x12: ExtVal = ext_mul(x11, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:218)
let x13: ExtVal = ext_mul(x12, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:219)
let x14: ExtVal = ext_mul(x13, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:220)
let x15: ExtVal = ext_mul(x14, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:221)
let x16: ExtVal = ext_mul(x15, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:222)
let x17: ExtVal = ext_mul(x16, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:223)
let x18: ExtVal = ext_mul(x17, arg2_0);
// builtin MakeExt
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:239)
let x19: ExtVal = ext_add(ExtVal(arg0.witness[decode(0u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x20: ExtVal = ext_add(ExtVal(arg0.witness[decode(268435454u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x21: ExtVal = ext_add(ExtVal(arg0.witness[decode(536870908u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x22: ExtVal = ext_add(ExtVal(arg0.witness[decode(805306362u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x23: ExtVal = ext_add(ExtVal(arg0.witness[decode(1073741816u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x24: ExtVal = ext_add(ExtVal(arg0.witness[decode(1342177270u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x25: ExtVal = ext_add(ExtVal(arg0.witness[decode(1610612724u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x26: ExtVal = ext_add(ExtVal(arg0.witness[decode(1879048178u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x27: ExtVal = ext_add(ExtVal(arg0.witness[decode(134217711u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x28: ExtVal = ext_add(ExtVal(arg0.witness[decode(402653165u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x29: ExtVal = ext_add(ExtVal(arg0.witness[decode(671088619u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x30: ExtVal = ext_add(ExtVal(arg0.witness[decode(939524073u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x31: ExtVal = ext_add(ExtVal(arg0.witness[decode(1207959527u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x32: ExtVal = ext_add(ExtVal(arg0.witness[decode(1476394981u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x33: ExtVal = ext_add(ExtVal(arg0.witness[decode(1744830435u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x34: ExtVal = ext_add(ExtVal(arg0.witness[decode(2013265889u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtAdd
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:238)
let x35: ExtVal = ext_add(ext_add(ext_mul(x19, ExtVal(268435454u, 0u, 0u, 0u)), ExtVal(0u, 0u, 0u, 0u)), ext_mul(x5, x20));
let x36: ExtVal = ext_add(ext_add(ext_add(x35, ext_mul(x6, x21)), ext_mul(x7, x22)), ext_mul(x8, x23));
let x37: ExtVal = ext_add(ext_add(ext_add(x36, ext_mul(x9, x24)), ext_mul(x10, x25)), ext_mul(x11, x26));
let x38: ExtVal = ext_add(ext_add(ext_add(x37, ext_mul(x12, x27)), ext_mul(x13, x28)), ext_mul(x14, x29));
let x39: ExtVal = ext_add(ext_add(ext_add(x38, ext_mul(x15, x30)), ext_mul(x16, x31)), ext_mul(x17, x32));
let x40: ExtVal = ext_add(ext_add(x39, ext_mul(x18, x33)), ext_mul(ext_mul(x18, arg2_0), x34));
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:246)
let x41: ExtVal = ext_add(arg1_0.poly._super, x40);
// BigIntPolyOpAddTotal(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:282)
let x42: NondetExtRegStruct = exec_ExtReg(ext_mul(x4, arg1_0.term._super), lookup_BigIntPolyOpAddTotalLayout_tmp(layout3));
// builtin ExtAdd
// BigIntPolyOpAddTotal(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:286)
let x43: ExtVal = ext_add(arg1_0.total._super, ext_mul(x42._super, x41));
// BigIntPolyOpAddTotal(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:283)
let x44: BigIntAccumStateStruct = exec_BigIntAccumState(ExtVal(0u, 0u, 0u, 0u), ExtVal(268435454u, 0u, 0u, 0u), x43, lookup_BigIntPolyOpAddTotalLayout__super(layout3));
return x44;
}

fn exec_BigIntPolyOpCarry1(arg0: BigIntTopStateStruct, arg1_0: BigIntAccumStateStruct, arg2_0: ExtVal, layout3: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// builtin ExtMul
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:210)
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:236)
// BigIntPolyOpCarry1(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:292)
let x4: ExtVal = ext_mul(arg2_0, ExtVal(268435454u, 0u, 0u, 0u));
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:211)
let x5: ExtVal = ext_mul(x4, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:212)
let x6: ExtVal = ext_mul(x5, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:213)
let x7: ExtVal = ext_mul(x6, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:214)
let x8: ExtVal = ext_mul(x7, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:215)
let x9: ExtVal = ext_mul(x8, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:216)
let x10: ExtVal = ext_mul(x9, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:217)
let x11: ExtVal = ext_mul(x10, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:218)
let x12: ExtVal = ext_mul(x11, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:219)
let x13: ExtVal = ext_mul(x12, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:220)
let x14: ExtVal = ext_mul(x13, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:221)
let x15: ExtVal = ext_mul(x14, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:222)
let x16: ExtVal = ext_mul(x15, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:223)
let x17: ExtVal = ext_mul(x16, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:224)
let x18: ExtVal = ext_mul(x17, arg2_0);
// builtin MakeExt
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:239)
let x19: ExtVal = ext_add(ExtVal(arg0.witness[decode(0u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x20: ExtVal = ext_add(ExtVal(arg0.witness[decode(268435454u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x21: ExtVal = ext_add(ExtVal(arg0.witness[decode(536870908u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x22: ExtVal = ext_add(ExtVal(arg0.witness[decode(805306362u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x23: ExtVal = ext_add(ExtVal(arg0.witness[decode(1073741816u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x24: ExtVal = ext_add(ExtVal(arg0.witness[decode(1342177270u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x25: ExtVal = ext_add(ExtVal(arg0.witness[decode(1610612724u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x26: ExtVal = ext_add(ExtVal(arg0.witness[decode(1879048178u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x27: ExtVal = ext_add(ExtVal(arg0.witness[decode(134217711u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x28: ExtVal = ext_add(ExtVal(arg0.witness[decode(402653165u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x29: ExtVal = ext_add(ExtVal(arg0.witness[decode(671088619u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x30: ExtVal = ext_add(ExtVal(arg0.witness[decode(939524073u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x31: ExtVal = ext_add(ExtVal(arg0.witness[decode(1207959527u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x32: ExtVal = ext_add(ExtVal(arg0.witness[decode(1476394981u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x33: ExtVal = ext_add(ExtVal(arg0.witness[decode(1744830435u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x34: ExtVal = ext_add(ExtVal(arg0.witness[decode(2013265889u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtAdd
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:238)
let x35: ExtVal = ext_add(ext_add(ext_mul(x19, ExtVal(268435454u, 0u, 0u, 0u)), ExtVal(0u, 0u, 0u, 0u)), ext_mul(x4, x20));
let x36: ExtVal = ext_add(ext_add(ext_add(x35, ext_mul(x5, x21)), ext_mul(x6, x22)), ext_mul(x7, x23));
let x37: ExtVal = ext_add(ext_add(ext_add(x36, ext_mul(x8, x24)), ext_mul(x9, x25)), ext_mul(x10, x26));
let x38: ExtVal = ext_add(ext_add(ext_add(x37, ext_mul(x11, x27)), ext_mul(x12, x28)), ext_mul(x13, x29));
let x39: ExtVal = ext_add(ext_add(ext_add(x38, ext_mul(x14, x30)), ext_mul(x15, x31)), ext_mul(x16, x32));
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:242)
let x40: ExtVal = ext_add(ext_add(ext_mul(x4, ExtVal(134217455u, 0u, 0u, 0u)), ExtVal(134217455u, 0u, 0u, 0u)), ext_mul(x5, ExtVal(134217455u, 0u, 0u, 0u)));
let x41: ExtVal = ext_add(ext_add(ext_add(x40, ext_mul(x6, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x7, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x8, ExtVal(134217455u, 0u, 0u, 0u)));
let x42: ExtVal = ext_add(ext_add(ext_add(x41, ext_mul(x9, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x10, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x11, ExtVal(134217455u, 0u, 0u, 0u)));
let x43: ExtVal = ext_add(ext_add(ext_add(x42, ext_mul(x12, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x13, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x14, ExtVal(134217455u, 0u, 0u, 0u)));
let x44: ExtVal = ext_add(ext_add(ext_add(x43, ext_mul(x15, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x16, ExtVal(134217455u, 0u, 0u, 0u))), ext_mul(x17, ExtVal(134217455u, 0u, 0u, 0u)));
// builtin ExtSub
// BigIntPolyOpCarry1(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:294)
let x45: ExtVal = ext_sub(ext_add(ext_add(x39, ext_mul(x17, x33)), ext_mul(x18, x34)), ext_add(x44, ext_mul(x18, ExtVal(134217455u, 0u, 0u, 0u))));
// builtin ExtAdd
let x46: ExtVal = ext_add(arg1_0.poly._super, ext_mul(x45, ExtVal(1073706872u, 0u, 0u, 0u)));
// BigIntPolyOpCarry1(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:293)
let x47: BigIntAccumStateStruct = exec_BigIntAccumState(x46, arg1_0.term._super, arg1_0.total._super, layout3);
return x47;
}

fn exec_BigIntPolyOpCarry2(arg0: BigIntTopStateStruct, arg1_0: BigIntAccumStateStruct, arg2_0: ExtVal, layout3: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// builtin ExtMul
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:210)
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:236)
// BigIntPolyOpCarry2(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:302)
let x4: ExtVal = ext_mul(arg2_0, ExtVal(268435454u, 0u, 0u, 0u));
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:211)
let x5: ExtVal = ext_mul(x4, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:212)
let x6: ExtVal = ext_mul(x5, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:213)
let x7: ExtVal = ext_mul(x6, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:214)
let x8: ExtVal = ext_mul(x7, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:215)
let x9: ExtVal = ext_mul(x8, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:216)
let x10: ExtVal = ext_mul(x9, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:217)
let x11: ExtVal = ext_mul(x10, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:218)
let x12: ExtVal = ext_mul(x11, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:219)
let x13: ExtVal = ext_mul(x12, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:220)
let x14: ExtVal = ext_mul(x13, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:221)
let x15: ExtVal = ext_mul(x14, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:222)
let x16: ExtVal = ext_mul(x15, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:223)
let x17: ExtVal = ext_mul(x16, arg2_0);
// builtin MakeExt
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:239)
let x18: ExtVal = ext_add(ExtVal(arg0.witness[decode(0u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x19: ExtVal = ext_add(ExtVal(arg0.witness[decode(268435454u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x20: ExtVal = ext_add(ExtVal(arg0.witness[decode(536870908u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x21: ExtVal = ext_add(ExtVal(arg0.witness[decode(805306362u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x22: ExtVal = ext_add(ExtVal(arg0.witness[decode(1073741816u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x23: ExtVal = ext_add(ExtVal(arg0.witness[decode(1342177270u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x24: ExtVal = ext_add(ExtVal(arg0.witness[decode(1610612724u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x25: ExtVal = ext_add(ExtVal(arg0.witness[decode(1879048178u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x26: ExtVal = ext_add(ExtVal(arg0.witness[decode(134217711u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x27: ExtVal = ext_add(ExtVal(arg0.witness[decode(402653165u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x28: ExtVal = ext_add(ExtVal(arg0.witness[decode(671088619u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x29: ExtVal = ext_add(ExtVal(arg0.witness[decode(939524073u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x30: ExtVal = ext_add(ExtVal(arg0.witness[decode(1207959527u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x31: ExtVal = ext_add(ExtVal(arg0.witness[decode(1476394981u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x32: ExtVal = ext_add(ExtVal(arg0.witness[decode(1744830435u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x33: ExtVal = ext_add(ExtVal(arg0.witness[decode(2013265889u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtAdd
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:238)
let x34: ExtVal = ext_add(ext_add(ext_mul(x18, ExtVal(268435454u, 0u, 0u, 0u)), ExtVal(0u, 0u, 0u, 0u)), ext_mul(x4, x19));
let x35: ExtVal = ext_add(ext_add(ext_add(x34, ext_mul(x5, x20)), ext_mul(x6, x21)), ext_mul(x7, x22));
let x36: ExtVal = ext_add(ext_add(ext_add(x35, ext_mul(x8, x23)), ext_mul(x9, x24)), ext_mul(x10, x25));
let x37: ExtVal = ext_add(ext_add(ext_add(x36, ext_mul(x11, x26)), ext_mul(x12, x27)), ext_mul(x13, x28));
let x38: ExtVal = ext_add(ext_add(ext_add(x37, ext_mul(x14, x29)), ext_mul(x15, x30)), ext_mul(x16, x31));
let x39: ExtVal = ext_add(ext_add(x38, ext_mul(x17, x32)), ext_mul(ext_mul(x17, arg2_0), x33));
// BigIntPolyOpCarry2(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:304)
let x40: ExtVal = ext_add(arg1_0.poly._super, ext_mul(x39, ExtVal(268434910u, 0u, 0u, 0u)));
// BigIntPolyOpCarry2(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:303)
let x41: BigIntAccumStateStruct = exec_BigIntAccumState(x40, arg1_0.term._super, arg1_0.total._super, layout3);
return x41;
}

fn exec_BigIntPolyOpEqz(arg0: BigIntTopStateStruct, arg1_0: BigIntAccumStateStruct, arg2_0: ExtVal, layout3: BoundLayout_BigIntAccumStateLayout) -> BigIntAccumStateStruct {
// builtin ExtMul
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:210)
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:236)
// BigIntPolyOpEqz(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:312)
let x4: ExtVal = ext_mul(arg2_0, ExtVal(268435454u, 0u, 0u, 0u));
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:211)
let x5: ExtVal = ext_mul(x4, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:212)
let x6: ExtVal = ext_mul(x5, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:213)
let x7: ExtVal = ext_mul(x6, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:214)
let x8: ExtVal = ext_mul(x7, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:215)
let x9: ExtVal = ext_mul(x8, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:216)
let x10: ExtVal = ext_mul(x9, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:217)
let x11: ExtVal = ext_mul(x10, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:218)
let x12: ExtVal = ext_mul(x11, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:219)
let x13: ExtVal = ext_mul(x12, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:220)
let x14: ExtVal = ext_mul(x13, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:221)
let x15: ExtVal = ext_mul(x14, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:222)
let x16: ExtVal = ext_mul(x15, arg2_0);
// BigIntAccumPowers(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:223)
let x17: ExtVal = ext_mul(x16, arg2_0);
// builtin MakeExt
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:239)
let x18: ExtVal = ext_add(ExtVal(arg0.witness[decode(0u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x19: ExtVal = ext_add(ExtVal(arg0.witness[decode(268435454u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x20: ExtVal = ext_add(ExtVal(arg0.witness[decode(536870908u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x21: ExtVal = ext_add(ExtVal(arg0.witness[decode(805306362u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x22: ExtVal = ext_add(ExtVal(arg0.witness[decode(1073741816u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x23: ExtVal = ext_add(ExtVal(arg0.witness[decode(1342177270u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x24: ExtVal = ext_add(ExtVal(arg0.witness[decode(1610612724u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x25: ExtVal = ext_add(ExtVal(arg0.witness[decode(1879048178u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x26: ExtVal = ext_add(ExtVal(arg0.witness[decode(134217711u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x27: ExtVal = ext_add(ExtVal(arg0.witness[decode(402653165u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x28: ExtVal = ext_add(ExtVal(arg0.witness[decode(671088619u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x29: ExtVal = ext_add(ExtVal(arg0.witness[decode(939524073u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x30: ExtVal = ext_add(ExtVal(arg0.witness[decode(1207959527u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x31: ExtVal = ext_add(ExtVal(arg0.witness[decode(1476394981u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x32: ExtVal = ext_add(ExtVal(arg0.witness[decode(1744830435u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
let x33: ExtVal = ext_add(ExtVal(arg0.witness[decode(2013265889u)], 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u));
// builtin ExtAdd
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:238)
let x34: ExtVal = ext_add(ext_add(ext_mul(x18, ExtVal(268435454u, 0u, 0u, 0u)), ExtVal(0u, 0u, 0u, 0u)), ext_mul(x4, x19));
let x35: ExtVal = ext_add(ext_add(ext_add(x34, ext_mul(x5, x20)), ext_mul(x6, x21)), ext_mul(x7, x22));
let x36: ExtVal = ext_add(ext_add(ext_add(x35, ext_mul(x8, x23)), ext_mul(x9, x24)), ext_mul(x10, x25));
let x37: ExtVal = ext_add(ext_add(ext_add(x36, ext_mul(x11, x26)), ext_mul(x12, x27)), ext_mul(x13, x28));
let x38: ExtVal = ext_add(ext_add(ext_add(x37, ext_mul(x14, x29)), ext_mul(x15, x30)), ext_mul(x16, x31));
let x39: ExtVal = ext_add(ext_add(x38, ext_mul(x17, x32)), ext_mul(ext_mul(x17, arg2_0), x33));
// BigIntAccumStep(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:246)
let x40: ExtVal = ext_add(arg1_0.poly._super, x39);
// BigIntPolyOpEqz(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:314)
let x41: ExtVal = ext_add(arg1_0.total._super, ext_mul(x40, ext_sub(x4, ExtVal(268434910u, 0u, 0u, 0u))));
// builtin EqzExt
// BigIntPolyOpEqz(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:315)
eqz_ext(x41);
// BigIntPolyOpEqz(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:316)
let x42: BigIntAccumStateStruct = exec_BigIntAccumState(ExtVal(0u, 0u, 0u, 0u), ExtVal(268435454u, 0u, 0u, 0u), ExtVal(0u, 0u, 0u, 0u), layout3);
return x42;
}

fn exec_BigIntAccum(arg0: BigIntTopStateStruct, arg1_0: ExtVal1Array, layout2: BoundLayout_BigIntAccumLayout) -> BigIntAccumStruct {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:195)
let x3: OneHot_7_Struct = exec_OneHot_7_(arg0.polyOp, lookup_BigIntAccumLayout_polyOp(layout2));
var x4: BigIntAccumStateStruct;
if ((x3._super[decode(0u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:198)
let x5: BigIntAccumStateStruct = exec_BigIntPolyOpNop(lookup_BigIntAccumStateLayout_0_arm0(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x5;
} else if ((x3._super[decode(268435454u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:199)
let x6: BigIntAccumStateStruct = back_BigIntAccumState(1, lookup_BigIntAccumLayout_state(layout2));
let x7: BigIntAccumStateStruct = exec_BigIntPolyOpShift(arg0, x6, arg1_0[decode(0u)], lookup_BigIntAccumStateLayout_0_arm1(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x7;
} else if ((x3._super[decode(536870908u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:200)
let x8: BigIntAccumStateStruct = back_BigIntAccumState(1, lookup_BigIntAccumLayout_state(layout2));
let x9: BigIntAccumStateStruct = exec_BigIntPolyOpSetTerm(arg0, x8, arg1_0[decode(0u)], lookup_BigIntAccumStateLayout_0_arm2(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x9;
} else if ((x3._super[decode(805306362u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:201)
let x10: BigIntAccumStateStruct = back_BigIntAccumState(1, lookup_BigIntAccumLayout_state(layout2));
let x11: BigIntAccumStateStruct = exec_BigIntPolyOpAddTotal(arg0, x10, arg1_0[decode(0u)], lookup_BigIntAccumStateLayout_0_arm3(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x11;
} else if ((x3._super[decode(1073741816u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:202)
let x12: BigIntAccumStateStruct = back_BigIntAccumState(1, lookup_BigIntAccumLayout_state(layout2));
let x13: BigIntAccumStateStruct = exec_BigIntPolyOpCarry1(arg0, x12, arg1_0[decode(0u)], lookup_BigIntAccumStateLayout_0_arm4(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x13;
} else if ((x3._super[decode(1342177270u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:203)
let x14: BigIntAccumStateStruct = back_BigIntAccumState(1, lookup_BigIntAccumLayout_state(layout2));
let x15: BigIntAccumStateStruct = exec_BigIntPolyOpCarry2(arg0, x14, arg1_0[decode(0u)], lookup_BigIntAccumStateLayout_0_arm5(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x15;
} else if ((x3._super[decode(1610612724u)]._super) != 0u) {
// BigIntAccum(zirgen/circuit/rv32im/v2/dsl/inst_bigint.zir:204)
let x16: BigIntAccumStateStruct = back_BigIntAccumState(1, lookup_BigIntAccumLayout_state(layout2));
let x17: BigIntAccumStateStruct = exec_BigIntPolyOpEqz(arg0, x16, arg1_0[decode(0u)], lookup_BigIntAccumStateLayout_0_arm6(lookup_BigIntAccumLayout_stateRedef(layout2)));
x4 = x17;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return BigIntAccumStruct(0u);
}

fn exec_Accum(arg0: BigIntTopStateStruct, arg1_0: ExtVal1Array, layout2: BoundLayout_AccumLayout) -> AccumStruct {
// Accum(zirgen/circuit/rv32im/v2/dsl/top.zir:100)
let x3: BigIntAccumStruct = exec_BigIntAccum(arg0, arg1_0, lookup_AccumLayout__0(layout2));
return AccumStruct(0u);
}

fn exec_TopExtractArm5(layout0: BoundLayout_TopLayout, global1: u32) -> BigIntTopStateStruct {

let x2: BoundLayout__globalLayout = BoundLayout__globalLayout(kLayoutGlobal, global1);
// builtin Sub
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:49)
let x3: Val = sub(268435454u, load(lookup_NondetRegLayout__super(lookup_TopLayout_isFirstCycle(layout0)), 0));
var x4: NondetRegStruct;
if ((load(lookup_NondetRegLayout__super(lookup_TopLayout_isFirstCycle(layout0)), 0)) != 0u) {
// builtin NondetReg
// Reg(<preamble>:5)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:51)
let x5: NondetRegStruct = NondetRegStruct(load(lookup_NondetRegLayout__super(lookup_TopCycleLayout_arm0(lookup_TopLayout_cycleRedef(layout0))), 0));
x4 = x5;
} else if ((x3) != 0u) {
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:53)
let x6: NondetRegStruct = NondetRegStruct(load(lookup_NondetRegLayout__super(lookup_TopCycleLayout_arm1(lookup_TopLayout_cycleRedef(layout0))), 0));
x4 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:56)
let x7: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextPcLow(layout0));
// builtin Mul
let x8: Val = mul(x3, x7._super);
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:57)
let x9: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextPcHigh(layout0));
// builtin Mul
let x10: Val = mul(x3, x9._super);
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:58)
let x11: ValU32Struct = ValU32Struct(x8, x10);
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:62)
let x12: NondetRegStruct = back_Reg(1, lookup_TopLayout_nextMachineMode(layout0));
// builtin Add
let x13: Val = add(mul(x3, x12._super), load(lookup_NondetRegLayout__super(lookup_TopLayout_isFirstCycle(layout0)), 0));
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
// SimpleOp(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:78)
// OpADD(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:91)
// Misc0(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:33)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:74)
let x14: Val = add(x8, 1073741816u);
// DenormedValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:20)
let x15: DenormedValU32Struct = DenormedValU32Struct(x14, x10);
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// FinalizeMisc(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:26)
// Misc0(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:42)
let x16: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
// DenormedValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:20)
// Denorm(zirgen/circuit/rv32im/v2/dsl/u32.zir:38)
// CmpOp(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:86)
// OpBEQ(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:161)
// Misc1(zirgen/circuit/rv32im/v2/dsl/inst_misc.zir:54)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:75)
let x17: DenormedValU32Struct = DenormedValU32Struct(0u, 0u);
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// IllegalMulOp(zirgen/circuit/rv32im/v2/dsl/inst_mul.zir:19)
// Mul0(zirgen/circuit/rv32im/v2/dsl/inst_mul.zir:32)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:77)
let x18: ValU32Struct = ValU32Struct(0u, 0u);
// builtin Component
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:98)
// OpSRL(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:136)
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:26)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:78)
let x19: ComponentStruct = ComponentStruct(0u);
// ECallOutput(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:12)
// IllegalECall(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:21)
// ECall0(zirgen/circuit/rv32im/v2/dsl/inst_ecall.zir:211)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:82)
let x20: ECallOutputStruct = ECallOutputStruct(0u, 0u, 0u, 0u);
var x21: InstOutputBaseStruct;


// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:36)
// NondetU16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:42)
// U16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:50)
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:65)
// DecodeInst(zirgen/circuit/rv32im/v2/dsl/inst.zir:27)
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:10)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:51)
// Top(zirgen/circuit/rv32im/v2/dsl/top.zir:79)
let x823: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_upperDiff(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))))), 0), 268295646u);
extern_noop();
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:69)
let x824: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_med14(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))))), 0), 268295646u);
extern_noop();
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:44)
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:12)
let x825: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0), 268295646u);
extern_noop();
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:50)
let x826: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0), 268295646u);
extern_noop();
// U16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:50)
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:85)
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:13)
let x827: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_upperDiff(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0), 268295646u);
extern_noop();
// AddrDecomposeBits(zirgen/circuit/rv32im/v2/dsl/u32.zir:89)
let x828: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_med14(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0), 268295646u);
extern_noop();
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// GetData(zirgen/circuit/rv32im/v2/dsl/mem.zir:36)
// MemoryRead(zirgen/circuit/rv32im/v2/dsl/mem.zir:92)
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:14)
let x829: ValU32Struct = ValU32Struct(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0), load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0));
// builtin Sub
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:89)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:53)
let x830: Val = sub(268435454u, load(lookup_NondetRegLayout__super(lookup_AddrDecomposeBitsLayout_low0(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))), 0));
// builtin Mul
// OpLH(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:99)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:54)
let x831: Val = mul(load(lookup_NondetRegLayout__super(lookup_AddrDecomposeBitsLayout_low1(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))), 0), load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0));
// builtin Sub
let x832: Val = sub(268435454u, load(lookup_NondetRegLayout__super(lookup_AddrDecomposeBitsLayout_low1(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))), 0));
// builtin Add
let x833: Val = add(x831, mul(x832, load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))), 0)));
var x834: ValU32Struct;
if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(0u))), 0)) != 0u) {
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:12)
// NondetU8Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:18)
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:34)
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:88)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:53)
let x835: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte0(lookup_OpLBLayout_bytes(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0), 268434910u);
extern_noop();
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:35)
let x836: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte1(lookup_OpLBLayout_bytes(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0), 268434910u);
extern_noop();
// builtin Mul
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:89)
let x837: Val = mul(load(lookup_NondetRegLayout__super(lookup_AddrDecomposeBitsLayout_low0(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))), 0), load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte1(lookup_OpLBLayout_bytes(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0));
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:12)
// NondetU8Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:18)
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:91)
let x838: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_OpLBLayout_low7x2(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))))), 0), 268434910u);
extern_noop();
// builtin Mul
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:93)
let x839: Val = mul(load(lookup_NondetRegLayout__super(lookup_OpLBLayout_highBit(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0), 2013126657u);
let x840: Val = mul(load(lookup_NondetRegLayout__super(lookup_OpLBLayout_highBit(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0), 2013126113u);
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
let x841: ValU32Struct = ValU32Struct(add(add(x837, mul(x830, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte0(lookup_OpLBLayout_bytes(lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0))), x839), x840);
x834 = x841;
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(268435454u))), 0)) != 0u) {
// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:36)
// NondetU16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:42)
// OpLH(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:101)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:54)
let x842: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_OpLHLayout_low15x2(lookup_Mem0OutputArm1Layout__super(lookup_Mem0OutputLayout_arm1(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))))), 0), 268295646u);
extern_noop();
// builtin Mul
// OpLH(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:103)
let x843: Val = mul(load(lookup_NondetRegLayout__super(lookup_OpLHLayout_highBit(lookup_Mem0OutputArm1Layout__super(lookup_Mem0OutputLayout_arm1(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0), 2013126113u);
x834 = ValU32Struct(x833, x843);
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(536870908u))), 0)) != 0u) {
x834 = x829;
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(805306362u))), 0)) != 0u) {
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:12)
// NondetU8Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:18)
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:34)
// OpLBU(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:116)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:56)
let x844: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte0(lookup_OpLBULayout_bytes(lookup_Mem0OutputArm3Layout__super(lookup_Mem0OutputLayout_arm3(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0), 268434910u);
extern_noop();
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:35)
let x845: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte1(lookup_OpLBULayout_bytes(lookup_Mem0OutputArm3Layout__super(lookup_Mem0OutputLayout_arm3(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0), 268434910u);
extern_noop();
// builtin Mul
// OpLBU(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:117)
let x846: Val = mul(load(lookup_NondetRegLayout__super(lookup_AddrDecomposeBitsLayout_low0(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))), 0), load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte1(lookup_OpLBULayout_bytes(lookup_Mem0OutputArm3Layout__super(lookup_Mem0OutputLayout_arm3(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0));
x834 = ValU32Struct(add(x846, mul(x830, load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(lookup_NondetU8RegLayout_arg(lookup_SplitWordLayout_byte0(lookup_OpLBULayout_bytes(lookup_Mem0OutputArm3Layout__super(lookup_Mem0OutputLayout_arm3(lookup_Mem0Layout_output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0)))))))))), 0))), 0u);
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(1073741816u))), 0)) != 0u) {
x834 = ValU32Struct(x833, 0u);
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(1342177270u))), 0)) != 0u) {
x834 = x18;
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(1610612724u))), 0)) != 0u) {
x834 = x18;
} else if ((load(lookup_NondetRegLayout__super(subscript_NondetRegLayout8LayoutArray(lookup_OneHot_8_Layout__super(lookup_InstInputLayout_minorOnehot(lookup_TopLayout_instInput(layout0))), decode(1879048178u))), 0)) != 0u) {
x834 = x18;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
// ArgU16(zirgen/circuit/rv32im/v2/dsl/lookups.zir:36)
// NondetU16Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:42)
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:44)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:63)
let x847: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0), 268295646u);
extern_noop();
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:50)
let x848: Val = inRange(0u, load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0), 268295646u);
extern_noop();
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// NormalizeU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:56)
let x849: ValU32Struct = ValU32Struct(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0), load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(layout0))))))), 0));
x21 = InstOutputBaseStruct(x849, 805306266u, x13, x16);

return x21.topState;
}

fn execUser_AccumArm5(arg0: BoundLayout_TopLayout, arg1_0: ExtVal1Array, layout2: BoundLayout_AccumLayout, global3: u32) -> AccumStruct {
let x4: BigIntTopStateStruct = exec_TopExtractArm5(arg0, global3);
let x5: AccumStruct = exec_Accum(x4, arg1_0, layout2);
return x5;
}

fn exec_TopAccumArm5(arg0: BoundLayout_TopLayout, layout1: BoundLayout_LayoutAccumLayout, global2: u32, mix3: u32) -> ComponentStruct {

// zirgen/dsl/passes/GenerateAccum.cpp:524
let x4: BoundLayout__mixLayout = BoundLayout__mixLayout(kLayoutMix, mix3);
// zirgen/dsl/passes/GenerateAccum.cpp:553
let x5: ExtVal1Array = ExtVal1Array(load_ext(subscript_Reg1LayoutArray(lookup__accumLayout__user(lookup__mixLayout_randomness(x4)), 0), 0));
let x6: AccumStruct = execUser_AccumArm5(arg0, x5, lookup_LayoutAccumLayout_user(layout1), global2);
// zirgen/dsl/passes/GenerateAccum.cpp:622
let x7: ComponentStruct = ComponentStruct(0u);
var x8: ComponentStruct;


// zirgen/dsl/passes/GenerateAccum.cpp:145
let x922: ExtVal = ext_mul(load_ext(lookup_Arg_CycleArgLayout_cycle(lookup__accumLayout_cycleArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_cycle(lookup_DoCycleTableLayout_arg1(lookup_Mem0Layout__0(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x923: ExtVal = ext_add(x922, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x924: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_DoCycleTableLayout_arg1(lookup_Mem0Layout__0(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))), 0), 0u, 0u, 0u), ext_inv(x923));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x925: ExtVal = ext_add(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 19), 1), x924);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x926: ExtVal = ext_mul(load_ext(lookup_Arg_CycleArgLayout_cycle(lookup__accumLayout_cycleArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_cycle(lookup_DoCycleTableLayout_arg2(lookup_Mem0Layout__0(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x927: ExtVal = ext_add(x926, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x928: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_DoCycleTableLayout_arg2(lookup_Mem0Layout__0(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))), 0), 0u, 0u, 0u), ext_inv(x927));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x929: ExtVal = ext_mul(x923, x927);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x930: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_DoCycleTableLayout_arg1(lookup_Mem0Layout__0(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))), 0), 0u, 0u, 0u), x927);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x931: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_upperDiff(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x932: ExtVal = ext_add(x931, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x933: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_upperDiff(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x932));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x934: ExtVal = ext_add(ext_add(x925, x928), x933);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 0), x934);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x935: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 0), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 19), 1));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x936: ExtVal = ext_sub(ext_sub(ext_mul(x935, ext_mul(x929, x932)), ext_mul(x930, x932)), ext_mul(ext_mul(x923, ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_DoCycleTableLayout_arg2(lookup_Mem0Layout__0(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))), 0), 0u, 0u, 0u)), x932));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x936, ext_mul(x929, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_upperDiff(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x937: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_med14(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x938: ExtVal = ext_add(x937, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x939: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_med14(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x938));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x940: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x941: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x942: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x943: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x944: ExtVal = ext_add(ext_add(ext_add(x940, x941), x942), x943);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x945: ExtVal = ext_add(x944, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x946: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x945));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x947: ExtVal = ext_mul(x938, x945);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x948: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeLayout_med14(lookup_DecodeInstLayout_pcAddr(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), x945);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x949: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x950: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x951: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x952: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x953: ExtVal = ext_add(ext_add(ext_add(x949, x950), x951), x952);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x954: ExtVal = ext_add(x953, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x955: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x954));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x956: ExtVal = ext_add(ext_add(ext_add(x934, x939), x946), x955);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 1), x956);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x957: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 1), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 0), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x958: ExtVal = ext_sub(ext_sub(ext_mul(x957, ext_mul(x947, x954)), ext_mul(x948, x954)), ext_mul(ext_mul(x938, ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u)), x954));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x958, ext_mul(x947, ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x959: ExtVal = ext_mul(load_ext(lookup_Arg_CycleArgLayout_cycle(lookup__accumLayout_cycleArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_cycle(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x960: ExtVal = ext_add(x959, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x961: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))))), 0), 0u, 0u, 0u), ext_inv(x960));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x962: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x963: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x964: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x965: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x966: ExtVal = ext_add(ext_add(ext_add(x962, x963), x964), x965);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x967: ExtVal = ext_add(x966, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x968: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x967));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x969: ExtVal = ext_mul(x960, x967);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x970: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_DecodeInstLayout_loadInst(lookup_MemLoadInputLayout_decoded(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))))), 0), 0u, 0u, 0u), x967);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x971: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x972: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x973: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
let x974: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x975: ExtVal = ext_add(ext_add(ext_add(x971, x972), x973), x974);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x976: ExtVal = ext_add(x975, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x977: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x976));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x978: ExtVal = ext_add(ext_add(ext_add(x956, x961), x968), x977);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 2), x978);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x979: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 2), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 1), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x980: ExtVal = ext_sub(ext_sub(ext_mul(x979, ext_mul(x969, x976)), ext_mul(x970, x976)), ext_mul(ext_mul(x960, ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u)), x976));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x980, ext_mul(x969, ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x981: ExtVal = ext_mul(load_ext(lookup_Arg_CycleArgLayout_cycle(lookup__accumLayout_cycleArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_cycle(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x982: ExtVal = ext_add(x981, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x983: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))))), 0), 0u, 0u, 0u), ext_inv(x982));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x984: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x985: ExtVal = ext_add(x984, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x986: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x985));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x987: ExtVal = ext_mul(x982, x985);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x988: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_ReadRegLayout__super(lookup_MemLoadInputLayout_rs1(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))))), 0), 0u, 0u, 0u), x985);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x989: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x990: ExtVal = ext_add(x989, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x991: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x990));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x992: ExtVal = ext_add(ext_add(ext_add(x978, x983), x986), x991);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 3), x992);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x993: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 3), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 2), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x994: ExtVal = ext_sub(ext_sub(ext_mul(x993, ext_mul(x987, x990)), ext_mul(x988, x990)), ext_mul(ext_mul(x982, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u)), x990));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x994, ext_mul(x987, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_MemLoadInputLayout_addrU32(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x995: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_upperDiff(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x996: ExtVal = ext_add(x995, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x997: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_upperDiff(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x996));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x998: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_med14(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x999: ExtVal = ext_add(x998, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1000: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_med14(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x999));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x1001: ExtVal = ext_mul(x996, x999);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x1002: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_upperDiff(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), x999);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1003: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1004: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1005: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1006: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x1007: ExtVal = ext_add(ext_add(ext_add(x1003, x1004), x1005), x1006);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1008: ExtVal = ext_add(x1007, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1009: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x1008));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x1010: ExtVal = ext_add(ext_add(ext_add(x992, x997), x1000), x1009);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 4), x1010);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x1011: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 4), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 3), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x1012: ExtVal = ext_sub(ext_sub(ext_mul(x1011, ext_mul(x1001, x1008)), ext_mul(x1002, x1008)), ext_mul(ext_mul(x996, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_AddrDecomposeBitsLayout_med14(lookup_MemLoadInputLayout_addr(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u)), x1008));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x1012, ext_mul(x1001, ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1013: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1014: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1015: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1016: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x1017: ExtVal = ext_add(ext_add(ext_add(x1013, x1014), x1015), x1016);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1018: ExtVal = ext_add(x1017, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1019: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x1018));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1020: ExtVal = ext_mul(load_ext(lookup_Arg_CycleArgLayout_cycle(lookup__accumLayout_cycleArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_cycle(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1021: ExtVal = ext_add(x1020, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1022: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x1021));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x1023: ExtVal = ext_mul(x1018, x1021);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x1024: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryReadLayout_io(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), x1021);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1025: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU8Layout_val(lookup__accumLayout_argU8(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 0))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1026: ExtVal = ext_add(x1025, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1027: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 0))), 0), 0u, 0u, 0u), ext_inv(x1026));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x1028: ExtVal = ext_add(ext_add(ext_add(x1010, x1019), x1022), x1027);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 5), x1028);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x1029: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 5), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 4), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x1030: ExtVal = ext_sub(ext_sub(ext_mul(x1029, ext_mul(x1023, x1026)), ext_mul(x1024, x1026)), ext_mul(ext_mul(x1018, ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryReadLayout__0(lookup_MemLoadInputLayout_data(lookup_Mem0Layout_input(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u)), x1026));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x1030, ext_mul(x1023, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 0))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1031: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU8Layout_val(lookup__accumLayout_argU8(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 1))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1032: ExtVal = ext_add(x1031, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1033: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 1))), 0), 0u, 0u, 0u), ext_inv(x1032));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1034: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU8Layout_val(lookup__accumLayout_argU8(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_val(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 2))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1035: ExtVal = ext_add(x1034, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1036: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 2))), 0), 0u, 0u, 0u), ext_inv(x1035));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x1037: ExtVal = ext_mul(x1032, x1035);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x1038: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 1))), 0), 0u, 0u, 0u), x1035);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1039: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(subscript_ArgU16Layout1LayoutArray(lookup__Arguments_Mem0OutputLayout_argU16(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 0))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1040: ExtVal = ext_add(x1039, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1041: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(subscript_ArgU16Layout1LayoutArray(lookup__Arguments_Mem0OutputLayout_argU16(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 0))), 0), 0u, 0u, 0u), ext_inv(x1040));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x1042: ExtVal = ext_add(ext_add(ext_add(x1028, x1033), x1036), x1041);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 6), x1042);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x1043: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 6), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 5), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x1044: ExtVal = ext_sub(ext_sub(ext_mul(x1043, ext_mul(x1037, x1040)), ext_mul(x1038, x1040)), ext_mul(ext_mul(x1032, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(subscript_ArgU8Layout3LayoutArray(lookup__Arguments_Mem0OutputLayout_argU8(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 2))), 0), 0u, 0u, 0u)), x1040));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x1044, ext_mul(x1037, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(subscript_ArgU16Layout1LayoutArray(lookup__Arguments_Mem0OutputLayout_argU16(lookup_Mem0Layout__arguments_Mem0Output(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))), 0))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1045: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_oldTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1046: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_oldTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1047: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_oldTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1048: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_oldTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x1049: ExtVal = ext_add(ext_add(ext_add(x1045, x1046), x1047), x1048);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1050: ExtVal = ext_add(x1049, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1051: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x1050));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1052: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_addr(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_addr(lookup_MemoryIOLayout_newTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1053: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_cycle(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_cycle(lookup_MemoryIOLayout_newTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1054: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataLow(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataLow(lookup_MemoryIOLayout_newTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
let x1055: ExtVal = ext_mul(load_ext(lookup_Arg_MemoryArgLayout_dataHigh(lookup__accumLayout_memoryArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_dataHigh(lookup_MemoryIOLayout_newTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:156
let x1056: ExtVal = ext_add(ext_add(ext_add(x1052, x1053), x1054), x1055);
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1057: ExtVal = ext_add(x1056, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1058: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), ext_inv(x1057));
// zirgen/dsl/passes/GenerateAccum.cpp:216
let x1059: ExtVal = ext_mul(x1050, x1057);
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x1060: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_oldTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u), x1057);
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1061: ExtVal = ext_mul(load_ext(lookup_Arg_CycleArgLayout_cycle(lookup__accumLayout_cycleArg(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_cycle(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryWriteLayout__0(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1062: ExtVal = ext_add(x1061, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1063: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryWriteLayout__0(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u), ext_inv(x1062));
// zirgen/dsl/passes/GenerateAccum.cpp:240
let x1064: ExtVal = ext_add(ext_add(ext_add(x1042, x1051), x1058), x1063);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 7), x1064);
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x1065: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 7), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 6), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x1066: ExtVal = ext_sub(ext_sub(ext_mul(x1065, ext_mul(x1059, x1062)), ext_mul(x1060, x1062)), ext_mul(ext_mul(x1050, ExtVal(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_MemoryIOLayout_newTxn(lookup_MemoryWriteLayout_io(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0)))))))), 0), 0u, 0u, 0u)), x1062));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(ext_sub(x1066, ext_mul(x1059, ExtVal(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_IsCycleLayout_arg(lookup_IsForwardLayout__0(lookup_MemoryWriteLayout__0(lookup_WriteRdLayout__0(lookup_Mem0Layout__1(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))))), 0), 0u, 0u, 0u))));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1067: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1068: ExtVal = ext_add(x1067, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1069: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))), 0), 0u, 0u, 0u), ext_inv(x1068));
// zirgen/dsl/passes/GenerateAccum.cpp:145
let x1070: ExtVal = ext_mul(load_ext(lookup_Arg_ArgU16Layout_val(lookup__accumLayout_argU16(lookup__mixLayout_randomness(x4))), 0), ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_val(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))), 0), 0u, 0u, 0u));
// zirgen/dsl/passes/GenerateAccum.cpp:237
let x1071: ExtVal = ext_add(x1070, load_ext(lookup__accumLayout__offset(lookup__mixLayout_randomness(x4)), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:239
let x1072: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))), 0), 0u, 0u, 0u), ext_inv(x1071));
// zirgen/dsl/passes/GenerateAccum.cpp:222
let x1073: ExtVal = ext_mul(ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_low16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))), 0), 0u, 0u, 0u), x1071);
// zirgen/dsl/passes/GenerateAccum.cpp:188
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 8), ext_add(ext_add(x1064, x1069), x1072));
// zirgen/dsl/passes/GenerateAccum.cpp:176
let x1074: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 8), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 7), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:179
let x1075: ExtVal = ext_sub(ext_sub(ext_mul(x1074, ext_mul(x1068, x1071)), x1073), ext_mul(x1068, ExtVal(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_NondetU16RegLayout_arg(lookup_NormalizeU32Layout_high16(lookup_Mem0Layout_pcAdd(lookup_TopInstResultLayout_arm5(lookup_TopLayout_instResult(arg0))))))), 0), 0u, 0u, 0u)));
// zirgen/dsl/passes/GenerateAccum.cpp:181
eqz_ext(x1075);
// zirgen/dsl/passes/GenerateAccum.cpp:122
store_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 19), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 8), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:124
let x1076: ExtVal = ext_sub(load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 19), 0), load_ext(subscript_Reg20LayoutArray(lookup_LayoutAccumLayout_columns(layout1), 8), 0));
// zirgen/dsl/passes/GenerateAccum.cpp:125
eqz_ext(x1076);
x8 = x7;

return x7;
}

fn step_TopAccumArm5(accum0: u32, data1: u32, global2: u32, mix3: u32) {
let x4: BoundLayout_TopLayout = BoundLayout_TopLayout(kLayout_Top, data1);
let x5: BoundLayout_LayoutAccumLayout = BoundLayout_LayoutAccumLayout(kLayout_TopAccum, accum0);
let x6: ComponentStruct = exec_TopAccumArm5(x4, x5, global2, mix3);
return;
}

@compute @workgroup_size(64)
fn topaccum_arm5_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  cycle = gid.x;
  if (cycle >= params.data_rows) { return; }
  step_TopAccumArm5(buf_accum, buf_data, buf_global, buf_mix);
}
