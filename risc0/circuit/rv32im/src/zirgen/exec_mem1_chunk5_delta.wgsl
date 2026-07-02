fn merge_ReadSourceRegsStruct(a: ReadSourceRegsStruct, b: ReadSourceRegsStruct) -> ReadSourceRegsStruct {
  return ReadSourceRegsStruct(ValU32Struct((a.rs1.low | b.rs1.low), (a.rs1.high | b.rs1.high)), ValU32Struct((a.rs2.low | b.rs2.low), (a.rs2.high | b.rs2.high)));
}
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
fn exec_ReadSourceRegsChunk1(arg0: NondetRegStruct, arg1_0: InstInputStruct, arg2_0: DecoderStruct, layout3: BoundLayout_ReadSourceRegsLayout) -> ReadSourceRegsStruct {
// builtin Isz
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:47)
let x4: Val = isz(sub(arg2_0.rs1, arg2_0.rs2));
let x5: NondetRegStruct = exec_NondetReg(x4, lookup_ReadSourceRegsLayout_isSameReg(layout3));
// builtin Sub
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:49)
let x6: Val = sub(268435454u, x5._super);
eqz(mul(x5._super, x6));
var x7: SourceRegsStruct;
if ((x6) != 0u) {
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:59)
let x8: GetDataStruct = exec_ReadReg(arg0, arg1_0, arg2_0.rs1, lookup_ReadSourceRegsSourceRegsArm1_SuperLayout__0(lookup_ReadSourceRegsSourceRegsLayout_arm1(lookup_ReadSourceRegsLayout_sourceRegs(layout3))));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:60)
let x9: GetDataStruct = exec_ReadReg(arg0, arg1_0, arg2_0.rs2, lookup_ReadSourceRegsSourceRegsArm1_SuperLayout__1(lookup_ReadSourceRegsSourceRegsLayout_arm1(lookup_ReadSourceRegsLayout_sourceRegs(layout3))));
x7 = SourceRegsStruct(x8._super, x9._super);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x10: ReadSourceRegsStruct;
if ((x6) != 0u) {
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:62)
let x11: NondetRegStruct = exec_Reg(x7.rs1.low, lookup_ReadSourceRegsLayout_rs1Low(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:63)
let x12: NondetRegStruct = exec_Reg(x7.rs1.high, lookup_ReadSourceRegsLayout_rs1High(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:64)
let x13: NondetRegStruct = exec_Reg(x7.rs2.low, lookup_ReadSourceRegsLayout_rs2Low(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:65)
let x14: NondetRegStruct = exec_Reg(x7.rs2.high, lookup_ReadSourceRegsLayout_rs2High(layout3));
// ReadSourceRegs(zirgen/circuit/rv32im/v2/dsl/inst.zir:46)
let x15: ReadSourceRegsStruct = ReadSourceRegsStruct(ValU32Struct(x11._super, x12._super), ValU32Struct(x13._super, x14._super));
x10 = x15;
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x10;
}
fn exec_MemStoreInput(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_MemStoreInputLayout) -> MemStoreInputStruct {
// MemStoreInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:18)
eqz(sub(arg1_0.state, 805306266u));
// MemStoreInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:20)
let x3: DecoderStruct = exec_DecodeInst(arg0, arg1_0, lookup_MemStoreInputLayout_decoded(layout2));
// MemStoreInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:21)
let x4: ReadSourceRegsStruct = exec_ReadSourceRegs_combined(arg0, arg1_0, x3, lookup_MemStoreInputLayout_sourceRegs(layout2));
// builtin Add
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
// MemStoreInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:24)
let x5: Val = add(x4.rs1.low, x3.immS.low);
let x6: Val = add(x4.rs1.high, x3.immS.high);
let x7: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x5, x6), lookup_MemStoreInputLayout_addrU32(layout2));
// MemStoreInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:25)
let x8: AddrDecomposeBitsStruct = exec_AddrDecomposeBits(x7._super, arg1_0.mode, lookup_MemStoreInputLayout_addr(layout2));
// MemStoreInput(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:26)
let x9: GetDataStruct = exec_MemoryRead(arg0, x8._super, lookup_MemStoreInputLayout_data(layout2));
return MemStoreInputStruct(x3, x4.rs2, x8, x9);
}
fn exec_MemStoreFinalize(arg0: NondetRegStruct, arg1_0: MemStoreInputStruct, arg2_0: ValU32Struct, layout3: BoundLayout_MemStoreFinalizeLayout) -> MemStoreFinalizeStruct {
// MemStoreFinalize(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:30)
let x4: MemoryWriteStruct = exec_MemoryWrite(arg0, arg1_0.addr._super, arg2_0, lookup_MemStoreFinalizeLayout__0(layout3));
return MemStoreFinalizeStruct(0u);
}
fn exec_Mem1Chunk5(arg0: NondetRegStruct, arg1_0: InstInputStruct, layout2: BoundLayout_Mem1Layout) -> InstOutputBaseStruct {
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:68)
let x3: DoCycleTableStruct = exec_DoCycleTable(arg0, lookup_Mem1Layout__0(layout2));
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:69)
let x4: MemStoreInputStruct = exec_MemStoreInput(arg0, arg1_0, lookup_Mem1Layout_input(layout2));
var x5: ValU32Struct;
if ((arg1_0.minorOnehot._super[decode(1342177270u)]._super) != 0u) {
// IllegalStoreOp(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:45)
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:76)
eqz(1744830467u);
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:70)
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra0(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra0(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra1(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra1(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra2(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra2(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0));
store(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra3(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0u);
eqz(load(lookup_NondetRegLayout__super(lookup_ArgU8Layout_count(lookup_Mem1OutputArm5Layout__extra3(lookup_Mem1OutputLayout_arm5(lookup_Mem1Layout_output(layout2))))), 0));
x5 = ValU32Struct(0u, 0u);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
var x6: InstOutputBaseStruct;
if ((arg1_0.minorOnehot._super[decode(1342177270u)]._super) != 0u) {
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:80)
let x7: MemStoreFinalizeStruct = exec_MemStoreFinalize(arg0, x4, x5, lookup_Mem1Layout__1(layout2));
// builtin Add
// AddU32(zirgen/circuit/rv32im/v2/dsl/u32.zir:27)
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:81)
let x8: Val = add(arg1_0.pcU32.low, 1073741816u);
let x9: NormalizeU32Struct = exec_NormalizeU32(DenormedValU32Struct(x8, arg1_0.pcU32.high), lookup_Mem1Layout_pcAdd(layout2));
// BigIntTopState(zirgen/circuit/rv32im/v2/dsl/inst.zir:107)
// BigIntTopStateNull(zirgen/circuit/rv32im/v2/dsl/inst.zir:114)
// InstOutput(zirgen/circuit/rv32im/v2/dsl/inst.zir:86)
// Mem1(zirgen/circuit/rv32im/v2/dsl/inst_mem.zir:82)
let x10: BigIntTopStateStruct = BigIntTopStateStruct(0u, 0u, Val16Array(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
x6 = InstOutputBaseStruct(x9._super, 805306266u, arg1_0.mode, x10);
} else {
  // TODO(wgsl): unreachable mux arm (no assert in WGSL)
}
return x6;
}
fn exec_ReadSourceRegs_combined(arg0: NondetRegStruct, arg1_0: InstInputStruct, arg2_0: DecoderStruct, layout3: BoundLayout_ReadSourceRegsLayout) -> ReadSourceRegsStruct {
  let r0 = exec_ReadSourceRegsChunk0(arg0, arg1_0, arg2_0, layout3);
  let r1 = exec_ReadSourceRegsChunk1(arg0, arg1_0, arg2_0, layout3);
  return merge_ReadSourceRegsStruct(r0, r1);
}
