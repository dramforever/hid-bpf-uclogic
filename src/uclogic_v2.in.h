// SPDX-License-Identifier: GPL-2.0-only OR BSD-2-Clause

// See "mk_template.h", and "hid_report_helpers.h" from udev-hid-bpf

UsagePage_Digitizers
Usage_Dig_Digitizer
CollectionApplication(
	ReportId(VENDOR_REPORT_ID)
	Usage_Dig_Stylus
	CollectionPhysical(
		LogicalMinimum_i8(0)
		LogicalMaximum_i8(1)
		ReportSize(1)
		Usage_Dig_TipSwitch
		Usage_Dig_BarrelSwitch
		Usage_Dig_SecondaryBarrelSwitch
		ReportCount(3)
		Input(Var|Abs)

		ReportCount(4) // Padding
		Input(Const)

		Usage_Dig_InRange
		ReportCount(1)
		Input(Var|Abs)

		ReportSize(24)
		ReportCount(1)
		PushPop(
			UsagePage_GenericDesktop
			Unit(in)
			UnitExponent(-3)

			LogicalMinimum_i32(0)
			0x27, FIELD(__u32, lmax_x) // LogicalMaximum_i32
			PhysicalMinimum_i32(0)
			0x47, FIELD(__u32, pmax_x) // PhysicalMaximum_i32
			Usage_GD_X
			Input(Var|Abs)

			LogicalMinimum_i32(0)
			0x27, FIELD(__u32, lmax_y) // LogicalMaximum_i32
			PhysicalMinimum_i32(0)
			0x47, FIELD(__u32, pmax_y) // PhysicalMaximum_i32
			Usage_GD_Y
			Input(Var|Abs)
		)

		LogicalMinimum_i16(0)
		0x26, FIELD(__u16, lmax_pressure) // LogicalMaximum_i16
		Usage_Dig_TipPressure
		ReportSize(16)
		ReportCount(1)
		Input(Var|Abs)

		ReportSize(8)
		ReportCount(2)
		PushPop(
			Unit(deg)
			UnitExponent(0)
			LogicalMinimum_i8(-60)
			PhysicalMinimum_i8(-60)
			LogicalMaximum_i8(60)
			PhysicalMaximum_i8(60)
			Usage_Dig_XTilt
			Usage_Dig_YTilt
			Input(Var|Abs)
		)
	)
)
UsagePage_GenericDesktop
Usage_GD_Keypad
CollectionApplication(
	ReportId(PAD_REPORT_ID)

	// Fake buttons, to convince consumer it's a tablet
	LogicalMinimum_i8(0)
	LogicalMaximum_i8(1)
	UsagePage_Digitizers
	Usage_Dig_TabletFunctionKeys
	CollectionPhysical(
		Usage_Dig_BarrelSwitch
		ReportCount(1)
		ReportSize(1)
		Input(Var|Abs)
		ReportCount(7)
		Input(Const)

		UsagePage_GenericDesktop
		Usage_GD_X
		Usage_GD_Y
		ReportCount(2)
		ReportSize(8)
		Input(Var|Abs)
	)

	UsagePage_Button
	UsageMinimum_i8(1)
	0x29, FIELD(__u8, num_btn_misc_1) // UsageMaximum_i8
	0x95, FIELD(__u8, num_btn_misc_2) // ReportCount
	ReportSize(1)
	Input(Var|Abs)

#ifdef use_btn_gamepad
	UsagePage_GenericDesktop
	Usage_GD_Gamepad

	UsagePage_Button
	UsageMinimum_i8(1)
	0x29, FIELD(__u8, num_btn_gamepad_1) // UsageMaximum_i8
	0x95, FIELD(__u8, num_btn_gamepad_2) // ReportCount
	Input(Var|Abs)
#endif

	0x95, FIELD(__u8, num_btn_padding) // ReportCount
	Input(Const)
)
