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
fn exec_MemLoadInput(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_MemLoadInputLayout) -> MemLoadInputStruct {
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:8)
eqz(sub(arg1_0.state, 805306266u));
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:10)
let x3: DecoderStruct = exec_DecodeInst(arg0, arg1_0, lookup_MemLoadInputLayout_decoded(layout2));
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:11)
let x4: GetDataStruct = exec_ReadReg(arg0, arg1_0, x3.rs1, lookup_MemLoadInputLayout_rs1(layout2));
// builtin Add
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:12)
let x5: Val = add(x4._super.low, x3.immI.low);
let x6: Val = add(x4._super.high, x3.immI.high);
let x7: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x5, x6), lookup_MemLoadInputLayout_addrU32(layout2));
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:13)
let x8: AddrDecomposeBitsStruct = exec_AddrDecomposeBits(x7._super, arg1_0.mode, lookup_MemLoadInputLayout_addr(layout2));
// MemLoadInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:14)
let x9: GetDataStruct = exec_MemoryRead(arg0, x8._super, lookup_MemLoadInputLayout_data(layout2));
return MemLoadInputStruct(arg1_0, x3, x8, x9);
}
fn exec_SplitWord(arg0: Val, layout1: BoundLayout_SplitWordLayout) -> SplitWordStruct {
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:34)
let x2: NondetRegStruct = exec_NondetU8Reg(bitAnd(arg0, 2013265377u), lookup_SplitWordLayout_byte0(layout1));
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:35)
let x3: NondetRegStruct = exec_NondetU8Reg(mul(bitAnd(arg0, 2013126657u), 16777216u), lookup_SplitWordLayout_byte1(layout1));
// builtin Add
// SplitWord(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:36)
let x4: Val = add(mul(x3._super, 268434910u), x2._super);
eqz(sub(arg0, x4));
return SplitWordStruct(x2, x3);
}
fn exec_OpLB(arg0: MemLoadInputStruct, layout1: BoundLayout_OpLBLayout) -> ValU32Struct {
// VerifyOpcodeF3(zirgen/circuit/rv32im/v2/dsl/inst.zir:96)
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:86)
let x2: Val = sub(arg0.decoded.opcode._super, 805306362u);
eqz(x2);
// VerifyOpcodeF3(zirgen/circuit/rv32im/v2/dsl/inst.zir:97)
eqz(arg0.decoded.func3);
// builtin Mul
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:87)
let x3: Val = mul(arg0.addr.low1._super, arg0.data._super.high);
// builtin Sub
let x4: Val = sub(268435454u, arg0.addr.low1._super);
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:88)
let x5: SplitWordStruct = exec_SplitWord(add(x3, mul(x4, arg0.data._super.low)), lookup_OpLBLayout_bytes(layout1));
// builtin Mul
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:89)
let x6: Val = mul(arg0.addr.low0._super, x5.byte1._super);
// builtin Sub
let x7: Val = sub(268435454u, arg0.addr.low0._super);
// builtin Add
let x8: Val = add(x6, mul(x7, x5.byte0._super));
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:90)
let x9: NondetRegStruct = exec_NondetBitReg(mul(bitAnd(x8, 134217455u), 33554432u), lookup_OpLBLayout_highBit(layout1));
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:91)
let x10: NondetRegStruct = exec_NondetU8Reg(mul(bitAnd(x8, 1879047922u), 536870908u), lookup_OpLBLayout_low7x2(layout1));
// builtin Add
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:92)
let x11: Val = add(mul(x9._super, 134217455u), mul(x10._super, 134217727u));
eqz(sub(x8, x11));
// ValU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:10)
// OpLB(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:93)
let x12: ValU32Struct = ValU32Struct(add(x8, mul(x9._super, 2013126657u)), mul(x9._super, 2013126113u));
return x12;
}
fn exec_Mem0Chunk0(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Mem0Layout) -> InstOutputBaseStruct {
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:50)
let x3: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Mem0Layout__0(layout2));
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:51)
let x4: MemLoadInputStruct = exec_MemLoadInput(arg0, arg1_0, lookup_Mem0Layout_input(layout2));
var x5: ValU32Struct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:53)
let x6: ValU32Struct = exec_OpLB(x4, lookup_Mem0OutputArm0Layout__super(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(layout2))));
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:52)
store(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Mem0OutputArm0Layout__extra0(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU16Layout_count(lookup_Mem0OutputArm0Layout__extra0(lookup_Mem0OutputLayout_arm0(lookup_Mem0Layout_output(layout2))))), 0));
x5 = x6;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x7: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(0u)]._super) != 0u) {
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:62)
let x8: WriteRdStruct = exec_WriteRd(arg0, x4.ii, x4.decoded, 268435454u, x5, lookup_Mem0Layout__1(layout2));
// builtin Add
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:63)
let x9: Val = add(arg1_0.pcU32.low, 1073741816u);
let x10: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x9, arg1_0.pcU32.high), lookup_Mem0Layout_pcAdd(layout2));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// Mem0(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:64)
let x11: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
x7 = InstOutputBaseStruct(x10._super, 805306266u, arg1_0.mode, x11);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x7;
}
