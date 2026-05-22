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
fn exec_NondetTwitReg(arg0: Val, layout1: BoundLayout_NondetRegLayout) -> NondetRegStruct {
// NondetTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:56)
let x2: NondetRegStruct = exec_NondetReg(arg0, layout1);
// builtin Mul
// AssertTwit(zirgen/circuit/rv32im/v2/dsl/bits.zir:38)
// NondetTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:57)
let x3: Val = mul(x2._super, sub(268435454u, x2._super));
let x4: Val = mul(mul(x3, sub(536870908u, x2._super)), sub(805306362u, x2._super));
eqz(x4);
return x2;
}
fn exec_NondetFakeTwitReg(arg0: Val, layout1: BoundLayout_NondetFakeTwitRegLayout) -> NondetFakeTwitRegStruct {
// NondetFakeTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:64)
let x2: NondetRegStruct = exec_NondetBitReg(bitAnd(arg0, 268435454u), lookup_NondetFakeTwitRegLayout_reg0(layout1));
// NondetFakeTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:65)
let x3: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(arg0, 536870908u), 134217727u), lookup_NondetFakeTwitRegLayout_reg1(layout1));
// builtin Add
// NondetFakeTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:66)
let x4: Val = add(mul(x3._super, 536870908u), x2._super);
return NondetFakeTwitRegStruct(x4);
}
fn exec_FakeTwitReg(arg0: Val, layout1: BoundLayout_NondetFakeTwitRegLayout) -> FakeTwitRegStruct {
// FakeTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:76)
let x2: NondetFakeTwitRegStruct = exec_NondetFakeTwitReg(arg0, layout1);
// FakeTwitReg(zirgen/circuit/rv32im/v2/dsl/bits.zir:77)
eqz(sub(arg0, x2._super));
return FakeTwitRegStruct(0u);
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
fn exec_ArgU8(arg0: Val, arg1_0: Val, layout2: BoundLayout_ArgU8Layout) -> ArgU8Struct {
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:9)
let x3: NondetRegStruct = exec_NondetReg(arg0, lookup_ArgU8Layout_count(layout2));
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:10)
let x4: NondetRegStruct = exec_NondetReg(arg1_0, lookup_ArgU8Layout_val(layout2));
// LookupDelta(zirgen/circuit/rv32im/v2/dsl/lookups.zir:4)
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:11)
extern_lookupDelta(134217711u, x4._super, x3._super);
// ArgU8(zirgen/circuit/rv32im/v2/dsl/lookups.zir:12)
let x5: Val = sub(268435454u, inRange(0u, x4._super, 268434910u));
extern_noop();
return ArgU8Struct(x3, x4);
}
fn exec_NondetU8Reg(arg0: Val, layout1: BoundLayout_NondetU8RegLayout) -> NondetRegStruct {
// NondetU8Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:18)
let x2: ArgU8Struct = exec_ArgU8(268435454u, arg0, lookup_NondetU8RegLayout_arg(layout1));
// NondetU8Reg(zirgen/circuit/rv32im/v2/dsl/lookups.zir:19)
let x3: Val = sub(x2.count._super, 268435454u);
eqz(x3);
return x2.val;
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
fn exec_ToBits_5_(arg0: Val, layout1: BoundLayout_ToBits_5_Layout) -> ToBits_5_Struct {
// ToBits(zirgen/circuit/rv32im/v2/dsl/po2.zir:24)
let x2: NondetRegStruct = exec_NondetBitReg(bitAnd(arg0, 268435454u), subscript_NondetRegLayout5LayoutArray(lookup_ToBits_5_Layout__super(layout1), decode(0u)));
let x3: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(arg0, 536870908u), 134217727u), subscript_NondetRegLayout5LayoutArray(lookup_ToBits_5_Layout__super(layout1), decode(268435454u)));
let x4: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(arg0, 1073741816u), 1073741824u), subscript_NondetRegLayout5LayoutArray(lookup_ToBits_5_Layout__super(layout1), decode(536870908u)));
let x5: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(arg0, 134217711u), 536870912u), subscript_NondetRegLayout5LayoutArray(lookup_ToBits_5_Layout__super(layout1), decode(805306362u)));
let x6: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(arg0, 268435422u), 268435456u), subscript_NondetRegLayout5LayoutArray(lookup_ToBits_5_Layout__super(layout1), decode(1073741816u)));
return ToBits_5_Struct(NondetRegStruct5Array(x2, x3, x4, x5, x6));
}
fn exec_DynPo2(arg0: Val, layout1: BoundLayout_DynPo2Layout) -> ValU32Struct {
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:38)
let x2: ToBits_5_Struct = exec_ToBits_5_(arg0, lookup_DynPo2Layout_low5(layout1));
// builtin Mul
// FromBits(zirgen/circuit/rv32im/v2/dsl/po2.zir:29)
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:39)
let x3: Val = mul(x2._super[decode(268435454u)]._super, 536870908u);
let x4: Val = mul(x2._super[decode(536870908u)]._super, 1073741816u);
let x5: Val = mul(x2._super[decode(805306362u)]._super, 134217711u);
let x6: Val = mul(x2._super[decode(1073741816u)]._super, 268435422u);
// builtin Add
let x7: Val = add(x2._super[decode(0u)]._super, x3);
let x8: Val = add(add(add(x7, x4), x5), x6);
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:40)
let x9: NondetU16RegStruct = exec_NondetU16Reg(mul(sub(arg0, x8), 134217728u), lookup_DynPo2Layout_checkU16(layout1));
// builtin Mul
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:41)
let x10: Val = mul(x9._super._super, 536870844u);
eqz(sub(add(x10, x8), arg0));
// builtin Mul
// CondMul(zirgen/circuit/rv32im/v2/dsl/po2.zir:33)
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:42)
let x11: Val = mul(x2._super[decode(0u)]._super, 536870908u);
// builtin Sub
let x12: Val = sub(268435454u, x2._super[decode(0u)]._super);
// builtin Add
let x13: Val = add(x11, x12);
// builtin Mul
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:43)
let x14: Val = mul(x2._super[decode(268435454u)]._super, x13);
// builtin Sub
let x15: Val = sub(268435454u, x2._super[decode(268435454u)]._super);
// builtin Add
let x16: Val = add(mul(x14, 1073741816u), mul(x15, x13));
// builtin Mul
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:44)
let x17: Val = mul(x2._super[decode(536870908u)]._super, x16);
// builtin Sub
let x18: Val = sub(268435454u, x2._super[decode(536870908u)]._super);
let x19: NondetRegStruct = exec_Reg(add(mul(x17, 268435422u), mul(x18, x16)), lookup_DynPo2Layout_b3(layout1));
// builtin Mul
// CondMul(zirgen/circuit/rv32im/v2/dsl/po2.zir:33)
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:45)
let x20: Val = mul(x2._super[decode(805306362u)]._super, x19._super);
// builtin Sub
let x21: Val = sub(268435454u, x2._super[decode(805306362u)]._super);
// builtin Add
let x22: Val = add(mul(x20, 268434910u), mul(x21, x19._super));
// builtin Sub
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:46)
let x23: Val = sub(268435454u, x2._super[decode(1073741816u)]._super);
let x24: NondetRegStruct = exec_Reg(mul(x23, x22), lookup_DynPo2Layout_low(layout1));
// builtin Mul
// DynPo2(zirgen/circuit/rv32im/v2/dsl/po2.zir:47)
let x25: Val = mul(x2._super[decode(1073741816u)]._super, x22);
let x26: NondetRegStruct = exec_Reg(x25, lookup_DynPo2Layout_high(layout1));
return ValU32Struct(x24._super, x26._super);
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
fn exec_AddrDecompose(arg0: ValU32Struct, arg1_0: Val, layout2: BoundLayout_AddrDecomposeLayout) -> AddrDecomposeStruct {
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:63)
let x3: NondetRegStruct = exec_NondetTwitReg(bitAnd(arg0.low, 805306362u), lookup_AddrDecomposeLayout_low2(layout2));
// builtin Mul
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:65)
let x4: Val = mul(sub(268435454u, arg1_0), 939419241u);
// builtin Sub
let x5: Val = sub(add(mul(arg1_0, 2013126113u), x4), arg0.high);
let x6: NondetU16RegStruct = exec_U16Reg(x5, lookup_AddrDecomposeLayout_upperDiff(layout2));
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:67)
let x7: NondetRegStruct = exec_IsZero(arg0.high, lookup_AddrDecomposeLayout__0(layout2));
eqz(x7._super);
// builtin Mul
// Div(<preamble>:19)
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:69)
let x8: Val = mul(sub(arg0.low, x3._super), 1073741824u);
let x9: NondetU16RegStruct = exec_NondetU16Reg(x8, lookup_AddrDecomposeLayout_med14(layout2));
// builtin Mul
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:71)
let x10: Val = mul(x9._super._super, 1073741816u);
eqz(sub(add(x10, x3._super), arg0.low));
// builtin Add
// AddrDecompose(zirgen/circuit/rv32im/v2/dsl/u32.zir:73)
let x11: Val = add(mul(arg0.high, 1073706872u), x9._super._super);
return AddrDecomposeStruct(x11, x3);
}
fn exec_CmpLessThanUnsigned(arg0: ValU32Struct, arg1_0: ValU32Struct, layout2: BoundLayout_CmpLessThanUnsignedLayout) -> CmpLessThanUnsignedStruct {
// builtin Sub
// SubU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:33)
// CmpLessThanUnsigned(zirgen/circuit/rv32im/v2/dsl/u32.zir:119)
let x3: Val = sub(add(arg0.low, 268295646u), arg1_0.low);
let x4: Val = sub(add(arg0.high, 2013126113u), arg1_0.high);
let x5: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x3, x4), lookup_CmpLessThanUnsignedLayout_diff(layout2));
// builtin Sub
// CmpLessThanUnsigned(zirgen/circuit/rv32im/v2/dsl/u32.zir:120)
let x6: Val = sub(268435454u, x5.highCarry._super);
return CmpLessThanUnsignedStruct(x6);
}
fn exec_Decoder(arg0: ValU32Struct, layout1: BoundLayout_DecoderLayout) -> DecoderStruct {
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:15)
let x2: Val = mul(bitAnd(arg0.high, 134147823u), 131072u);
let x3: NondetRegStruct = exec_NondetBitReg(x2, lookup_DecoderLayout__f7_6(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:16)
let x4: Val = mul(bitAnd(arg0.high, 1610560308u), 524288u);
let x5: NondetRegStruct = exec_NondetTwitReg(x4, lookup_DecoderLayout__f7_45(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:17)
let x6: Val = mul(bitAnd(arg0.high, 402640077u), 2097152u);
let x7: NondetRegStruct = exec_NondetTwitReg(x6, lookup_DecoderLayout__f7_23(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:18)
let x8: Val = mul(bitAnd(arg0.high, 1610609460u), 8388608u);
let x9: NondetRegStruct = exec_NondetTwitReg(x8, lookup_DecoderLayout__f7_01(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:19)
let x10: Val = mul(bitAnd(arg0.high, 402652365u), 33554432u);
let x11: NondetRegStruct = exec_NondetTwitReg(x10, lookup_DecoderLayout__rs2_34(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:20)
let x12: Val = mul(bitAnd(arg0.high, 1610612532u), 134217728u);
let x13: NondetRegStruct = exec_NondetTwitReg(x12, lookup_DecoderLayout__rs2_12(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:21)
let x14: Val = mul(bitAnd(arg0.high, 268435422u), 268435456u);
let x15: NondetRegStruct = exec_NondetBitReg(x14, lookup_DecoderLayout__rs2_0(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:22)
let x16: Val = mul(bitAnd(arg0.high, 1207959527u), 1073741824u);
let x17: NondetRegStruct = exec_NondetTwitReg(x16, lookup_DecoderLayout__rs1_34(layout1));
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:23)
let x18: NondetRegStruct = exec_NondetTwitReg(bitAnd(arg0.high, 805306362u), lookup_DecoderLayout__rs1_12(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:24)
let x19: Val = mul(bitAnd(arg0.low, 134147823u), 131072u);
let x20: NondetRegStruct = exec_NondetBitReg(x19, lookup_DecoderLayout__rs1_0(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:25)
let x21: Val = mul(bitAnd(arg0.low, 1073706872u), 262144u);
let x22: NondetRegStruct = exec_NondetBitReg(x21, lookup_DecoderLayout__f3_2(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:26)
let x23: Val = mul(bitAnd(arg0.low, 805280154u), 1048576u);
let x24: NondetRegStruct = exec_NondetTwitReg(x23, lookup_DecoderLayout__f3_01(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:27)
let x25: Val = mul(bitAnd(arg0.low, 1207952999u), 4194304u);
let x26: NondetRegStruct = exec_NondetTwitReg(x25, lookup_DecoderLayout__rd_34(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:28)
let x27: Val = mul(bitAnd(arg0.low, 805304730u), 16777216u);
let x28: NondetRegStruct = exec_NondetTwitReg(x27, lookup_DecoderLayout__rd_12(layout1));
// builtin Mul
// Div(<preamble>:19)
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:29)
let x29: Val = mul(bitAnd(arg0.low, 134217455u), 33554432u);
let x30: NondetRegStruct = exec_NondetBitReg(x29, lookup_DecoderLayout__rd_0(layout1));
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:34)
let x31: NondetRegStruct = exec_NondetReg(bitAnd(arg0.low, 1879047922u), lookup_DecoderLayout_opcode(layout1));
// builtin Add
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:37)
let x32: Val = add(mul(x3._super, 134147823u), mul(x5._super, 536853436u));
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:39)
let x33: Val = add(add(x32, mul(x7._super, 134213359u)), mul(x9._super, 536869820u));
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:41)
let x34: Val = add(add(x33, mul(x11._super, 134217455u)), mul(x13._super, 536870844u));
// builtin Mul
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:44)
let x35: Val = mul(x17._super, 1073741816u);
// builtin Add
let x36: Val = add(add(add(x34, mul(x15._super, 268435422u)), x35), x18._super);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:37)
eqz(sub(arg0.high, x36));
// builtin Mul
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:46)
let x37: Val = mul(x20._super, 134147823u);
// builtin Add
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:47)
let x38: Val = add(add(x37, mul(x22._super, 1073706872u)), mul(x24._super, 268426718u));
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:49)
let x39: Val = add(add(x38, mul(x26._super, 1073739640u)), mul(x28._super, 268434910u));
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:46)
let x40: Val = sub(arg0.low, add(add(x39, mul(x30._super, 134217455u)), x31._super));
eqz(x40);
// builtin Add
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:55)
let x41: Val = add(mul(x17._super, 134217711u), mul(x18._super, 536870908u));
// builtin Mul
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:56)
let x42: Val = mul(x11._super, 134217711u);
let x43: Val = mul(x13._super, 536870908u);
// builtin Add
let x44: Val = add(add(x42, x43), x15._super);
// builtin Mul
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:57)
let x45: Val = mul(x26._super, 134217711u);
let x46: Val = mul(x28._super, 536870908u);
// builtin Add
let x47: Val = add(add(x45, x46), x30._super);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:58)
let x48: Val = add(mul(x5._super, 268435422u), mul(x7._super, 1073741816u));
let x49: Val = add(x48, x9._super);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:59)
let x50: Val = add(mul(x3._super, 1073741688u), x49);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:60)
let x51: Val = add(mul(x22._super, 1073741816u), x24._super);
// builtin Mul
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:66)
let x52: Val = mul(x3._super, 2013134849u);
// builtin Add
let x53: Val = add(x52, mul(x50, 536870844u));
// builtin Mul
let x54: Val = mul(x3._super, 2013126113u);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:68)
let x55: Val = mul(x49, 536870844u);
// builtin Add
let x56: Val = add(add(add(x52, mul(x30._super, 134213359u)), x55), x45);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:71)
let x57: Val = add(add(x37, mul(x51, 268426718u)), mul(x15._super, 134213359u));
let x58: Val = add(add(add(x57, x55), x42), x43);
// Decoder(zirgen/circuit/rv32im/v2/dsl/decode.zir:72)
let x59: Val = add(mul(x3._super, 2013126145u), x35);
return DecoderStruct(x31, add(x41, x20._super), x44, x47, x50, x51, ValU32Struct(add(x53, x44), x54), ValU32Struct(add(x53, x47), x54), ValU32Struct(add(x56, x46), x54), ValU32Struct(x38, arg0.high), ValU32Struct(x58, add(x59, x18._super)));
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
fn exec_MemoryWrite(arg0: NondetRegStruct, arg1_0: Val, arg2_0: ValU32Struct, layout3: BoundLayout_MemoryWriteLayout) -> MemoryWriteStruct {
// builtin Add
// MemoryWrite(zirgen/circuit/rv32im/v2/dsl/mem.zir:97)
let x4: Val = add(mul(arg0._super, 536870908u), 268435454u);
let x5: MemoryIOStruct = exec_MemoryIO(x4, arg1_0, lookup_MemoryWriteLayout_io(layout3));
// MemoryWrite(zirgen/circuit/rv32im/v2/dsl/mem.zir:98)
let x6: IsForwardStruct = exec_IsForward(x5, lookup_MemoryWriteLayout__0(layout3));
// MemoryWrite(zirgen/circuit/rv32im/v2/dsl/mem.zir:99)
let x7: Val = sub(x5.newTxn.dataLow._super, arg2_0.low);
eqz(x7);
// MemoryWrite(zirgen/circuit/rv32im/v2/dsl/mem.zir:100)
let x8: Val = sub(x5.newTxn.dataHigh._super, arg2_0.high);
eqz(x8);
return MemoryWriteStruct(0u);
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
fn exec_DecodeInst(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_DecodeInstLayout) -> DecoderStruct {
// DecodeInst(zirgen/circuit/rv32im/v2/dsl/inst.zir:27)
let x3: AddrDecomposeStruct = exec_AddrDecompose(arg1_0.pcU32, arg1_0.mode, lookup_DecodeInstLayout_pcAddr(layout2));
// DecodeInst(zirgen/circuit/rv32im/v2/dsl/inst.zir:29)
eqz(x3.low2._super);
// DecodeInst(zirgen/circuit/rv32im/v2/dsl/inst.zir:31)
let x4: GetDataStruct = exec_MemoryRead(arg0, x3._super, lookup_DecodeInstLayout_loadInst(layout2));
// DecodeInst(zirgen/circuit/rv32im/v2/dsl/inst.zir:33)
let x5: DecoderStruct = exec_Decoder(x4._super, lookup_DecodeInstLayout__super(layout2));
return x5;
}
fn exec_ReadReg(arg0: NondetRegStruct, arg1_0: InstInputStruct, arg2_0: Val, layout3: BoundLayout_ReadRegLayout) -> GetDataStruct {
// builtin Mul
// ReadReg(zirgen/circuit/rv32im/v2/dsl/inst.zir:37)
let x4: Val = mul(sub(268435454u, arg1_0.mode), 259522493u);
// builtin Add
let x5: Val = add(mul(arg1_0.mode, 1735917570u), x4);
let x6: NondetRegStruct = exec_Reg(add(x5, arg2_0), lookup_ReadRegLayout_addr(layout3));
// ReadReg(zirgen/circuit/rv32im/v2/dsl/inst.zir:38)
let x7: GetDataStruct = exec_MemoryRead(arg0, x6._super, lookup_ReadRegLayout__super(layout3));
return x7;
}
fn exec_ReadSourceRegsChunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, arg2_0: DecoderStruct, layout3: BoundLayout_ReadSourceRegsLayout) -> ReadSourceRegsStruct {
// builtin Sub
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:47)
let x4: Val = sub(arg2_0.rs1, arg2_0.rs2);
let x5: NondetRegStruct = exec_NondetReg(isz(x4), lookup_ReadSourceRegsLayout_isSameReg(layout3));
// builtin Mul
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:49)
let x6: Val = mul(x5._super, sub(268435454u, x5._super));
eqz(x6);
var x7: SourceRegsStruct;
if ((x5._super) != 0u) {
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:52)
eqz(x4);
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:53)
let x8: GetDataStruct = exec_ReadReg(arg0, arg1_0, arg2_0.rs1, lookup_ReadSourceRegsSourceRegsArm0_SuperLayout_rboth(lookup_ReadSourceRegsSourceRegsArm0Layout__super(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3)))));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:50)
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ReadSourceRegsSourceRegsArm0Layout__extra0(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ReadSourceRegsSourceRegsArm0Layout__extra0(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ReadSourceRegsSourceRegsArm0Layout__extra1(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_MemoryArgLayout_count(lookup_ReadSourceRegsSourceRegsArm0Layout__extra1(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3))))), 0));
store(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_ReadSourceRegsSourceRegsArm0Layout__extra2(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_CycleArgLayout_count(lookup_ReadSourceRegsSourceRegsArm0Layout__extra2(lookup_ReadSourceRegsSourceRegsLayout_arm0(lookup_ReadSourceRegsLayout_sourceRegs(layout3))))), 0));
x7 = SourceRegsStruct(x8._super, x8._super);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x9: ReadSourceRegsStruct;
if ((x5._super) != 0u) {
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:62)
let x10: NondetRegStruct = exec_Reg(x7.rs1.low, lookup_ReadSourceRegsLayout_rs1Low(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:63)
let x11: NondetRegStruct = exec_Reg(x7.rs1.high, lookup_ReadSourceRegsLayout_rs1High(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:64)
let x12: NondetRegStruct = exec_Reg(x7.rs2.low, lookup_ReadSourceRegsLayout_rs2Low(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:65)
let x13: NondetRegStruct = exec_Reg(x7.rs2.high, lookup_ReadSourceRegsLayout_rs2High(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:46)
let x14: ReadSourceRegsStruct = ReadSourceRegsStruct(ValU32Struct(x10._super, x11._super), ValU32Struct(x12._super, x13._super));
x9 = x14;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x9;
}
fn exec_WriteRd(arg0: NondetRegStruct, arg1_0: InstInputStruct, arg2_0: DecoderStruct, arg3: Val, arg4: ValU32Struct, layout5: BoundLayout_WriteRdLayout) -> WriteRdStruct {
// WriteRd(zirgen/circuit/rv32im/v2/dsl/inst.zir:71)
let x6: NondetRegStruct = exec_IsZero(arg2_0.rd, lookup_WriteRdLayout_isRd0(layout5));
// builtin Mul
// WriteRd(zirgen/circuit/rv32im/v2/dsl/inst.zir:72)
let x7: Val = mul(sub(268435454u, x6._super), arg3);
// WriteRd(zirgen/circuit/rv32im/v2/dsl/inst.zir:74)
let x8: Val = mul(sub(268435454u, arg1_0.mode), 259522493u);
// builtin Add
let x9: Val = add(mul(arg1_0.mode, 1735917570u), x8);
// builtin Mul
let x10: Val = mul(sub(268435454u, x7), 1073741688u);
let x11: NondetRegStruct = exec_Reg(add(add(x9, x10), mul(x7, arg2_0.rd)), lookup_WriteRdLayout_writeAddr(layout5));
// WriteRd(zirgen/circuit/rv32im/v2/dsl/inst.zir:75)
let x12: MemoryWriteStruct = exec_MemoryWrite(arg0, x11._super, arg4, lookup_WriteRdLayout__0(layout5));
return WriteRdStruct(0u);
}
fn exec_ExpandU32(arg0: ValU32Struct, arg1_0: Val, layout2: BoundLayout_ExpandU32Layout) -> ExpandU32Struct {
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:51)
let x3: NondetRegStruct = exec_NondetU8Reg(bitAnd(arg0.low, 2013265377u), lookup_ExpandU32Layout_b0(layout2));
// builtin Mul
// Div(<preamble>:19)
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:52)
let x4: Val = mul(bitAnd(arg0.low, 2013126657u), 16777216u);
let x5: NondetRegStruct = exec_NondetU8Reg(x4, lookup_ExpandU32Layout_b1(layout2));
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:53)
let x6: NondetRegStruct = exec_NondetU8Reg(bitAnd(arg0.high, 2013265377u), lookup_ExpandU32Layout_b2(layout2));
// builtin Mul
// Div(<preamble>:19)
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:54)
let x7: Val = mul(bitAnd(arg0.high, 2013126657u), 16777216u);
let x8: NondetRegStruct = exec_NondetU8Reg(x7, lookup_ExpandU32Layout_b3(layout2));
// builtin Mul
// Div(<preamble>:19)
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:60)
let x9: Val = mul(bitAnd(arg0.high, 1878978834u), 33554432u);
let x10: NondetRegStruct = exec_NondetU8Reg(x9, lookup_ExpandU32Layout_b3Top7times2(layout2));
// builtin Mul
// Div(<preamble>:19)
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:61)
let x11: Val = mul(bitAnd(arg0.high, 134147823u), 131072u);
let x12: NondetRegStruct = exec_NondetBitReg(x11, lookup_ExpandU32Layout_topBit(layout2));
// builtin Add
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:63)
let x13: Val = add(x3._super, mul(x5._super, 268434910u));
eqz(sub(arg0.low, x13));
// builtin Add
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:64)
let x14: Val = add(x6._super, mul(x10._super, 134217455u));
eqz(sub(arg0.high, add(x14, mul(x12._super, 134147823u))));
// builtin Add
// ExpandU32(zirgen/circuit/rv32im/v2/dsl/mult.zir:68)
let x15: Val = add(mul(x10._super, 134217727u), mul(x12._super, 134217455u));
eqz(sub(x8._super, x15));
return ExpandU32Struct(x3, x5, x6, x8, mul(x12._super, arg1_0));
}
fn exec_SplitTotal(arg0: Val, layout1: BoundLayout_SplitTotalLayout) -> SplitTotalStruct {
// SplitTotal(zirgen/circuit/rv32im/v2/dsl/mult.zir:98)
let x2: NondetU16RegStruct = exec_NondetU16Reg(bitAnd(arg0, 2013126113u), lookup_SplitTotalLayout_out_(layout1));
// SplitTotal(zirgen/circuit/rv32im/v2/dsl/mult.zir:99)
let x3: NondetRegStruct = exec_NondetU8Reg(mul(bitAnd(arg0, 1977614337u), 65536u), lookup_SplitTotalLayout_carryByte(layout1));
// SplitTotal(zirgen/circuit/rv32im/v2/dsl/mult.zir:100)
let x4: NondetFakeTwitRegStruct = exec_NondetFakeTwitReg(mul(bitAnd(arg0, 1476395009u), 256u), lookup_SplitTotalLayout_carryExtra(layout1));
// builtin Add
// SplitTotal(zirgen/circuit/rv32im/v2/dsl/mult.zir:101)
let x5: Val = add(mul(x4._super, 232644062u), mul(x3._super, 268295646u));
eqz(sub(arg0, add(x5, x2._super._super)));
// builtin Add
// SplitTotal(zirgen/circuit/rv32im/v2/dsl/mult.zir:102)
let x6: Val = add(mul(x4._super, 268434910u), x3._super);
return SplitTotalStruct(x2, x6);
}
fn exec_MultiplyAccumulate(arg0: ValU32Struct, arg1_0: ValU32Struct, arg2_0: ValU32Struct, arg3: MultiplySettingsStruct, layout4: BoundLayout_MultiplyAccumulateLayout) -> MultiplyAccumulateStruct {
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:115)
let x5: Val = sub(268435454u, inRange(0u, arg3.aSigned, 536870908u));
extern_noop();
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:116)
let x6: Val = sub(268435454u, inRange(0u, arg3.bSigned, 536870908u));
extern_noop();
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:117)
let x7: Val = sub(268435454u, inRange(0u, arg3.cSigned, 536870908u));
extern_noop();
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:119)
let x8: ExpandU32Struct = exec_ExpandU32(arg0, arg3.aSigned, lookup_MultiplyAccumulateLayout_ax(layout4));
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:120)
let x9: ExpandU32Struct = exec_ExpandU32(arg1_0, arg3.bSigned, lookup_MultiplyAccumulateLayout_bx(layout4));
// builtin Mul
// Div(<preamble>:19)
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:122)
let x10: Val = mul(bitAnd(arg2_0.high, 134147823u), 131072u);
let x11: NondetRegStruct = exec_NondetBitReg(x10, lookup_MultiplyAccumulateLayout_cSign(layout4));
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:123)
let x12: Val = mul(bitAnd(arg2_0.high, 1878978290u), 536870908u);
let x13: NondetU16RegStruct = exec_NondetU16Reg(x12, lookup_MultiplyAccumulateLayout_cRestTimes2(layout4));
// builtin Mul
// Div(<preamble>:19)
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:124)
let x14: Val = mul(x13._super._super, 134217727u);
// builtin Add
let x15: Val = add(mul(x11._super, 134147823u), x14);
eqz(sub(arg2_0.high, x15));
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:128)
let x16: Val = mul(x8.b0._super, x9.b0._super);
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:129)
let x17: Val = mul(x8.b0._super, x9.b1._super);
let x18: Val = mul(x8.b1._super, x9.b0._super);
// builtin Add
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:128)
let x19: Val = add(add(arg2_0.low, x16), mul(add(x17, x18), 268434910u));
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:126)
let x20: SplitTotalStruct = exec_SplitTotal(x19, lookup_MultiplyAccumulateLayout_s0(layout4));
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:135)
let x21: Val = mul(x8.b0._super, x9.b2._super);
// builtin Add
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:134)
let x22: Val = add(add(arg2_0.high, x20.carry), x21);
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:135)
let x23: Val = mul(x8.b1._super, x9.b1._super);
let x24: Val = mul(x8.b2._super, x9.b0._super);
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:136)
let x25: Val = mul(x8.b0._super, x9.b3._super);
let x26: Val = mul(x8.b1._super, x9.b2._super);
let x27: Val = mul(x8.b2._super, x9.b1._super);
let x28: Val = mul(x8.b3._super, x9.b0._super);
// builtin Add
let x29: Val = add(add(add(x25, x26), x27), x28);
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:135)
let x30: Val = add(add(add(x22, x23), x24), mul(x29, 268434910u));
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:132)
let x31: SplitTotalStruct = exec_SplitTotal(x30, lookup_MultiplyAccumulateLayout_s1(layout4));
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:142)
let x32: Val = mul(mul(x11._super, 2013126113u), arg3.cSigned);
// builtin Add
let x33: Val = add(add(x31.carry, x32), 536591292u);
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:143)
let x34: Val = mul(x8.b1._super, 268434910u);
// builtin Add
let x35: Val = add(x8.b0._super, x34);
// builtin Mul
let x36: Val = mul(x9.b1._super, 268434910u);
// builtin Add
let x37: Val = add(x9.b0._super, x36);
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:144)
let x38: Val = mul(x8.b1._super, x9.b3._super);
// builtin Add
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:143)
let x39: Val = add(sub(sub(x33, mul(x35, x9.neg)), mul(x37, x8.neg)), x38);
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:144)
let x40: Val = mul(x8.b2._super, x9.b2._super);
let x41: Val = mul(x8.b3._super, x9.b1._super);
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:145)
let x42: Val = mul(x8.b2._super, x9.b3._super);
let x43: Val = mul(x8.b3._super, x9.b2._super);
// builtin Add
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:144)
let x44: Val = add(add(add(x39, x40), x41), mul(add(x42, x43), 268434910u));
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:140)
let x45: SplitTotalStruct = exec_SplitTotal(x44, lookup_MultiplyAccumulateLayout_s2(layout4));
// builtin Add
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:151)
let x46: Val = add(add(x45.carry, x32), 2012986305u);
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:152)
let x47: Val = mul(x8.b3._super, 268434910u);
// builtin Add
let x48: Val = add(x8.b2._super, x47);
// builtin Mul
let x49: Val = mul(x9.b3._super, 268434910u);
// builtin Add
let x50: Val = add(x9.b2._super, x49);
// builtin Mul
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:153)
let x51: Val = mul(x8.b3._super, x9.b3._super);
// builtin Add
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:152)
let x52: Val = add(sub(sub(x46, mul(x48, x9.neg)), mul(x50, x8.neg)), x51);
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:154)
let x53: NondetU16RegStruct = exec_NondetU16Reg(bitAnd(x52, 2013126113u), lookup_MultiplyAccumulateLayout_s3Out(layout4));
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:155)
let x54: FakeTwitRegStruct = exec_FakeTwitReg(mul(sub(x52, x53._super._super), 65536u), lookup_MultiplyAccumulateLayout_s3Carry(layout4));
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:156)
let x55: ValU32Struct = ValU32Struct(x20.out_._super._super, x31.out_._super._super);
// MultiplyAccumulate(zirgen/circuit/rv32im/v2/dsl/mult.zir:157)
let x56: ValU32Struct = ValU32Struct(x45.out_._super._super, x53._super._super);
return MultiplyAccumulateStruct(x55, x56, x9.neg);
}
fn exec_DivInput(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_DivInputLayout) -> DivInputStruct {
// DivInput(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:8)
eqz(sub(arg1_0.state, 805306266u));
// DivInput(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:10)
let x3: DecoderStruct = exec_DecodeInst(arg0, arg1_0, lookup_DivInputLayout_decoded(layout2));
// DivInput(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:11)
let x4: ReadSourceRegsStruct = exec_ReadSourceRegsChunk0(arg0, arg1_0, x3, lookup_DivInputLayout_sourceRegs(layout2));
return DivInputStruct(arg1_0, x3, x4.rs1, x4.rs2);
}
fn exec_DoDivChunk0(arg0: ValU32Struct, arg1_0: ValU32Struct, arg2_0: Val, arg3: Val, layout4: BoundLayout_DoDivLayout) -> DivideReturnStruct {
// Divide(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:45)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:49)
let x5_tuple = extern_divide(arg0.low, arg0.high, arg1_0.low, arg1_0.high, add(arg2_0, mul(arg3, 536870908u)));
let x5: Val = x5_tuple[0u];
let x6: Val = x5_tuple[1u];
let x7: Val = x5_tuple[2u];
let x8: Val = x5_tuple[3u];
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:52)
let x9: NondetRegStruct = exec_NondetReg(x5, lookup_DoDivLayout_quotLow(layout4));
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:53)
let x10: NondetRegStruct = exec_NondetReg(x6, lookup_DoDivLayout_quotHigh(layout4));
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:54)
let x11: ValU32Struct = ValU32Struct(x9._super, x10._super);
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:56)
let x12: NondetU16RegStruct = exec_NondetU16Reg(x7, lookup_DoDivLayout_remLow(layout4));
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:57)
let x13: NondetU16RegStruct = exec_NondetU16Reg(x8, lookup_DoDivLayout_remHigh(layout4));
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:58)
let x14: ValU32Struct = ValU32Struct(x12._super._super, x13._super._super);
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:62)
let x15: MultiplyAccumulateStruct = exec_MultiplyAccumulate(x11, arg1_0, x14, MultiplySettingsStruct(arg2_0, arg2_0, arg2_0), lookup_DoDivLayout_mul(layout4));
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:106)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:64)
let x16: Val = sub(x15.outLow.low, arg0.low);
eqz(x16);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:107)
let x17: Val = sub(x15.outLow.high, arg0.high);
eqz(x17);
// builtin Isz
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:66)
let x18: Val = isz(x15.outHigh.low);
let x19: NondetRegStruct = exec_NondetBitReg(sub(268435454u, x18), lookup_DoDivLayout_topBitType(layout4));
// builtin Mul
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:67)
let x20: Val = mul(x19._super, 2013126113u);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:106)
let x21: Val = sub(x15.outHigh.low, x20);
eqz(x21);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:107)
let x22: Val = sub(x15.outHigh.high, x20);
eqz(x22);
// builtin Mul
// Div(<preamble>:19)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:69)
let x23: Val = mul(bitAnd(arg0.high, 134147823u), 131072u);
let x24: NondetRegStruct = exec_NondetBitReg(x23, lookup_DoDivLayout_topNum(layout4));
// builtin Sub
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:71)
let x25: Val = sub(arg0.high, mul(x24._super, 134147823u));
let x26: NondetU16RegStruct = exec_U16Reg(mul(x25, 536870908u), lookup_DoDivLayout__0(layout4));
// builtin Mul
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:72)
let x27: Val = mul(x24._super, arg2_0);
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:76)
let x28: Val = mul(x15.bNeg, sub(268295646u, arg1_0.low));
// builtin Sub
let x29: Val = sub(268435454u, x15.bNeg);
// builtin Mul
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:77)
let x30: Val = mul(x15.bNeg, sub(2013126113u, arg1_0.high));
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:75)
let x31: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(add(x28, mul(x29, arg1_0.low)), add(x30, mul(x29, arg1_0.high))), lookup_DoDivLayout_denomAbs(layout4));
// builtin Sub
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:81)
let x32: Val = sub(268295646u, x12._super._super);
let x33: Val = sub(268435454u, x27);
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:82)
let x34: Val = sub(2013126113u, x13._super._super);
// DenormedValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:20)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:80)
let x35: DenormedValU32Struct = DenormedValU32Struct(add(mul(x27, x32), mul(x33, x12._super._super)), add(mul(x27, x34), mul(x33, x13._super._super)));
let x36: NormalizeU32Struct = exec_NormalizeU32(x35, lookup_DoDivLayout_remNormal(layout4));
// builtin Isz
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:90)
let x37: Val = isz(add(arg1_0.low, arg1_0.high));
let x38: NondetRegStruct = exec_NondetBitReg(x37, lookup_DoDivLayout_isZero(layout4));
// builtin Isz
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:93)
let x39: Val = isz(sub(arg0.high, 134147823u));
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:94)
let x40: Val = isz(sub(arg1_0.low, 2013126113u));
// builtin Mul
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:93)
let x41: Val = mul(mul(mul(arg2_0, isz(arg0.low)), x39), x40);
// builtin Isz
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:94)
let x42: Val = isz(sub(arg1_0.high, 2013126113u));
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:91)
let x43: NondetRegStruct = exec_NondetBitReg(mul(x41, x42), lookup_DoDivLayout_signedOverflowCase(layout4));
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:96)
let x44: CmpLessThanUnsignedStruct = exec_CmpLessThanUnsigned(x36._super, x31._super, lookup_DoDivLayout_lt(layout4));
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:106)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:101)
let x45: Val = sub(x12._super._super, arg0.low);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:107)
let x46: Val = sub(x13._super._super, arg0.high);
var x47: ComponentStruct;
if ((x38._super) != 0u) {
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:106)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:100)
eqz(arg1_0.low);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:107)
eqz(arg1_0.high);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:106)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:101)
eqz(x45);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:107)
eqz(x46);
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:106)
// DoDiv(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:102)
eqz(sub(x9._super, 2013126113u));
// AssertEqU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:107)
eqz(sub(x10._super, 2013126113u));
x47 = ComponentStruct(0u);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x48: DivideReturnStruct;
if ((x38._super) != 0u) {
x48 = DivideReturnStruct(x11, x14);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x48;
}
fn exec_OpSRL(arg0: DivInputStruct, layout1: BoundLayout_OpSRLLayout) -> ValU32Struct {
// VerifyOpcodeF3F7(zirgen/circuit/rv32im/v2/dsl/inst.zir:102)
// OpSRL(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:134)
let x2: Val = sub(arg0.decoded.opcode._super, 1610612628u);
eqz(x2);
// VerifyOpcodeF3F7(zirgen/circuit/rv32im/v2/dsl/inst.zir:103)
let x3: Val = sub(arg0.decoded.func3, 1342177270u);
eqz(x3);
// VerifyOpcodeF3F7(zirgen/circuit/rv32im/v2/dsl/inst.zir:104)
eqz(arg0.decoded.func7);
// OpSRL(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:135)
let x4: ValU32Struct = exec_DynPo2(arg0.rs2.low, lookup_OpSRLLayout_shiftMul(layout1));
// OpSRL(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:136)
let x5: DivideReturnStruct = exec_DoDivChunk0(arg0.rs1, x4, 0u, 0u, lookup_OpSRLLayout__0(layout1));
return x5.quot;
}
fn exec_Div0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Div0Layout) -> InstOutputBaseStruct {
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:23)
let x3: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Div0Layout__0(layout2));
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:24)
let x4: DivInputStruct = exec_DivInput(arg0, arg1_0, lookup_Div0Layout_input(layout2));
var x5: ValU32Struct;
if ((x4._super.minorOnehot._super[decode(0u)]._super) != 0u) {
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:26)
let x6: ValU32Struct = exec_OpSRL(x4, lookup_Div0MulOutputArm0Layout__super(lookup_Div0MulOutputLayout_arm0(lookup_Div0Layout_mulOutput(layout2))));
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:25)
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Div0MulOutputArm0Layout__extra0(lookup_Div0MulOutputLayout_arm0(lookup_Div0Layout_mulOutput(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Div0MulOutputArm0Layout__extra0(lookup_Div0MulOutputLayout_arm0(lookup_Div0Layout_mulOutput(layout2))))), 0));
x5 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x7: InstOutputBaseStruct;
if ((x4._super.minorOnehot._super[decode(0u)]._super) != 0u) {
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:35)
let x8: WriteRdStruct = exec_WriteRd(arg0, x4._super, x4.decoded, 268435454u, x5, lookup_Div0Layout__1(layout2));
// builtin Add
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:36)
let x9: Val = add(arg1_0.pcU32.low, 1073741816u);
let x10: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x9, arg1_0.pcU32.high), lookup_Div0Layout_pcAdd(layout2));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// Div0(zirgen/circuit/rv32im/v2/dsl/inst_div.zir:37)
let x11: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
x7 = InstOutputBaseStruct(x10._super, 805306266u, arg1_0.mode, x11);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x7;
}
