use fuseable::FuseableError;
use fuseable_derive::Fuseable;

script! {
    "hard resets the sensor and brings it into standby\n"
    Reset { test: u8 } => {
        read => (self) {
            Err(FuseableError::unsupported("read", fuseable::type_name(&self)))
        }
        write [value] => (self, sensor, sensor_io) {
            println!("writing {:?}", value);

            sensor_io.write_raw("reset", 1)?;

            std::thread::sleep(std::time::Duration::from_millis(10));

            sensor_io.write_raw("reset", 0)?;
            sensor.write_cooked("software_reset", 0)?;
            sensor.write_cooked("stream", 1)?;

            Ok(())
        }
    }
}

script! {
    "start up the sensor in default settings"
    Kick {} => {
        read => (self) {
            Err(FuseableError::unsupported("read", fuseable::type_name(&self)))
        }
        write [value] => (self, sensor, sensor_io) {
            let extclk = 24000000;
            // init
            // toggle reset (active low)
            sensor_io.write_raw("reset", 0x7)?;
            std::thread::sleep(std::time::Duration::from_millis(1));
            sensor_io.write_raw("reset", 0x0)?;
            std::thread::sleep(std::time::Duration::from_millis(1));
            sensor_io.write_raw("reset", 0x7)?;

            // magic init
            sensor.write_raw("magic_init_config", 0xa114)?;
            sensor.write_raw("magic_init_start", 0x0070)?;

            std::thread::sleep(std::time::Duration::from_millis(1));

            // check chip_version
            let chip_version = sensor.read_raw("chip_version_reg")?;
            // assert(chip_version == "0x2304");

            println!("chip_version {}", chip_version);
            println!("reserved_chiprev {}", sensor.read_raw("reserved_chiprev")?);
            println!("version {}", sensor.read_raw("test_data_red")?);

            /*
            write("magic_patch1", 0x0146);
            write("magic_patch2", 0x88bc);
            write("magic_patch3", 0xaa63);
            write("magic_patch4", 0x00a0);
            */

            fn gcd(mut a: usize, mut b: usize) -> usize {
                while b != 0 {
                    let old_b = b;
                    b = a % b;
                    a = old_b;
                }

                a
            }

            fn optimal_pll_config(extclk: usize, vco_target: usize) -> (usize, usize) {
                let vco_maximum = 768000000; // chip max
                // assert(vco_target < vco_maximum)
                let vco_max = vco_target;
                let vco_min = 384000000;

                let div_min = 1;
                let div_max = 64;

                let mul_min = 32;
                let mul_max = 384;

                let mut div = 0;
                let mut mul = 0;

                for vco in (vco_min..=vco_max).rev() {
                    let i = gcd(extclk, vco);
                    div = extclk / i;
                    mul = vco / i;

                    if (mul <= mul_max) && (mul >= mul_min) && (div <= div_max) {
                        break
                    }

                }

                (div, mul)
            }

            // pll config for 12bit, 4 lane hispi
            let vco_hispi_4lanes_12bit_clk = 588000000; // 588 MHz
            let (pre_pll_clk_div, pll_multiplier)
                = optimal_pll_config(extclk, vco_hispi_4lanes_12bit_clk);

            // taken from table in datasheet, no idea how to calculate on our own
            let vt_sys_clk_div =  2;
            let vt_pix_clk_div =  6;
            let op_sys_clk_div =  2;
            let op_pix_clk_div = 12;

            sensor.write_raw("vt_pix_clk_div", vt_pix_clk_div)?;
            sensor.write_raw("vt_sys_clk_div", vt_sys_clk_div)?;
            sensor.write_raw("pre_pll_clk_div", pre_pll_clk_div)?;
            sensor.write_raw("pll_multiplier", pll_multiplier)?;
            sensor.write_raw("op_pix_clk_div", op_pix_clk_div)?;
            sensor.write_raw("op_sys_clk_div", op_sys_clk_div)?;

            // pll lock time
            std::thread::sleep(std::time::Duration::from_millis(1));

            // data format setting
            // 0xc0c - 12bit raw uncompressed
            sensor.write_raw("data_format_bits", 0x0c0c)?;
            // serial output format
            // select hivcm (1V8)
            sensor.write_raw("datapath_select", 1 << 9)?;


            // hispi enable, test pattern all ones
            // write("hispi_control_status", int("0000 0011 1101 0100".replace(' ', ''), 2))
            // !!!! IMPORTANT  !!!! the 0x0400 bit toggles streaming s -> packetized sp
            sensor.write_raw("hispi_control_status", 0b1000_0100_0000_0000)?;
            sensor.write_raw("mipi_config_status", 0xc)?;

            // 0x0202 - 2 lane mipi
            // 0x0304 - 4 lane hispi
            sensor.write_raw("serial_format", 0x0304)?;

            // test pattern mode
            // 0   - no test pattern
            // 1   - solid color
            // 2   - solid color bars
            // 3   - fade to gray color bars
            // 256 - walking 1s
            sensor.write_raw("test_pattern_mode", 0)?;

            // unlock write to data_pedestal
            sensor.write_raw("reset_register", 0b10000)?;
            sensor.write_raw("test_raw_mode", 2)?;
            sensor.write_raw("data_pedestal", 0)?;

            // dubious, we have duplicate addresses for this one
            // sensor.write_register("dark_control", 0)?;

            sensor.write_raw("analog_gain", 0x0010)?;
            sensor.write_raw("global_gain", 0b0000000010000000)?;
            sensor.write_raw("coarse_integration_time", 1200)?;
            sensor.write_raw("fine_integration_time", 0)?;

            // reset hispi_timing
            sensor.write_raw("hispi_timing", 0b1_000_000_000_000_000)?;
            // streaming enable
            sensor.write_raw("mode_select", 1)?;

            Ok(())
        }
    }

}

script! {
    "hard resets the ar0331 and brings it into standby\n"
    ResetAR0331 { test: u8 } => {
        read => (self) {
            Err(FuseableError::unsupported("read", fuseable::type_name(&self)))
        }
        write [value] => (self, sensor, sensor_io) {
            println!("writing {:?}", value);

            sensor_io.write_raw("reset", 1)?;

            std::thread::sleep(std::time::Duration::from_millis(10));

            sensor_io.write_raw("reset", 0)?;
            sensor.write_cooked("software_reset", 0)?;
            sensor.write_cooked("stream", 1)?;

            Ok(())
        }
    }
}

script_set! {
    MicroR2Scripts => {
        "reset": Reset,
        "kick": Kick
    }
}
