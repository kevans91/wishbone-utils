
#[derive(Debug)]
pub enum RiscvCpuError {
    /// Someone tried to request an unrecognized feature file
    UnrecognizedFile(String /* requested filename */)
}

const THREADS_XML: &str = r#"<?xml version="1.0"?>
<threads>
</threads>"#;

#[derive(PartialEq)]
enum RiscvRegisterType {
    /// Normal CPU registers
    General,

    /// Arch-specific registers
    CSR,
}

impl RiscvRegisterType {
    fn feature_name(&self) -> &str {
        match *self {
            RiscvRegisterType::General => "org.gnu.gdb.riscv.cpu",
            RiscvRegisterType::CSR => "org.gnu.gdb.riscv.csr",
        }
    }

    fn group(&self) -> &str {
        match *self {
            RiscvRegisterType::General => "general",
            RiscvRegisterType::CSR => "csr",
        }
    }
}

struct RiscvRegister {

    /// Which register group this belongs to
    register_type: RiscvRegisterType,

    /// Index within its namespace (e.g. `ustatus` is a CSR with index 0x000,
    /// even though GDB registers are offset by 65, so GDB calls `ustatus` register 65.)
    index: u32,

    /// Architecture name
    name: String,

    /// Whether this register is present on this device
    present: bool,
}

impl RiscvRegister {
    pub fn general(index: u32, name: &str) -> RiscvRegister {
        RiscvRegister {
            register_type: RiscvRegisterType::General,
            index,
            name: name.to_string(),
            present: true,
        }
    }

    pub fn csr(index: u32, name: &str, present: bool) -> RiscvRegister {
        RiscvRegister {
            register_type: RiscvRegisterType::CSR,
            index,
            name: name.to_string(),
            present,
        }
    }
}

pub struct RiscvCpu {

    /// A list of all available registers on this CPU
    registers: Vec<RiscvRegister>,

    /// An XML representation of the register mapping
    target_xml: String,
}

impl RiscvCpu {
    pub fn new() -> Result<RiscvCpu, RiscvCpuError> {
        let registers = Self::make_registers();
        let target_xml = Self::make_target_xml(&registers);
        Ok(RiscvCpu {registers, target_xml})
    }

    fn make_registers() -> Vec<RiscvRegister> {
        let mut registers = vec![];

        // Add in general purpose registers x0 to x31
        for reg_num in 0..32 {
            registers.push(RiscvRegister::general(reg_num, &format!("x{}", reg_num)));
        }

        // Add the program counter
        registers.push(RiscvRegister::general(32, "pc"));

        // User trap setup
        registers.push(RiscvRegister::csr(0x000, "ustatus", false));
        registers.push(RiscvRegister::csr(0x004, "uie", false));
        registers.push(RiscvRegister::csr(0x005, "utvec", false));

        // User trap handling
        registers.push(RiscvRegister::csr(0x040, "uscratch", false));
        registers.push(RiscvRegister::csr(0x041, "uepc", false));
        registers.push(RiscvRegister::csr(0x042, "ucause", false));
        registers.push(RiscvRegister::csr(0x043, "utval", false));
        registers.push(RiscvRegister::csr(0x044, "uip", false));

        // User counter/timers
        registers.push(RiscvRegister::csr(0xc00, "cycle", false));
        registers.push(RiscvRegister::csr(0xc01, "time", false));
        registers.push(RiscvRegister::csr(0xc02, "instret", false));
        for hpmcounter_n in 3..32 {
            registers.push(RiscvRegister::csr(0xc00 + hpmcounter_n, &format!("hpmcounter{}", hpmcounter_n), false));
        }
        registers.push(RiscvRegister::csr(0xc80, "cycleh", false));
        registers.push(RiscvRegister::csr(0xc81, "timeh", false));
        registers.push(RiscvRegister::csr(0xc82, "instreth", false));
        for hpmcounter_n in 3..32 {
            registers.push(RiscvRegister::csr(0xc80 + hpmcounter_n, &format!("hpmcounter{}h", hpmcounter_n), false));
        }

        // Supervisor Trap Setup
        registers.push(RiscvRegister::csr(0x100, "sstatus", false));
        registers.push(RiscvRegister::csr(0x102, "sedeleg", false));
        registers.push(RiscvRegister::csr(0x103, "sideleg", false));
        registers.push(RiscvRegister::csr(0x104, "sie", false));
        registers.push(RiscvRegister::csr(0x105, "stvec", false));
        registers.push(RiscvRegister::csr(0x106, "scounteren", false));

        // Supervisor Trap Handling
        registers.push(RiscvRegister::csr(0x140, "sscratch", false));
        registers.push(RiscvRegister::csr(0x141, "sepc", false));
        registers.push(RiscvRegister::csr(0x142, "scause", false));
        registers.push(RiscvRegister::csr(0x143, "stval", false));
        registers.push(RiscvRegister::csr(0x144, "sip", false));

        // Supervisor protection and translation
        registers.push(RiscvRegister::csr(0x180, "satp", false));

        // Machine information registers
        registers.push(RiscvRegister::csr(0xf11, "mvendorid", false));
        registers.push(RiscvRegister::csr(0xf12, "marchid", false));
        registers.push(RiscvRegister::csr(0xf13, "mimpid", false));
        registers.push(RiscvRegister::csr(0xf14, "mhartid", false));

        // Machine trap setup
        registers.push(RiscvRegister::csr(0x300, "mstatus", false));
        registers.push(RiscvRegister::csr(0x301, "misa", false));
        registers.push(RiscvRegister::csr(0x302, "medeleg", false));
        registers.push(RiscvRegister::csr(0x303, "mideleg", false));
        registers.push(RiscvRegister::csr(0x304, "mie", false));
        registers.push(RiscvRegister::csr(0x305, "mtvec", false));
        registers.push(RiscvRegister::csr(0x306, "mcounteren", false));

        // Machine trap handling
        registers.push(RiscvRegister::csr(0x340, "mscratch", false));
        registers.push(RiscvRegister::csr(0x341, "mepc", false));
        registers.push(RiscvRegister::csr(0x342, "mcause", false));
        registers.push(RiscvRegister::csr(0x343, "mtval", false));
        registers.push(RiscvRegister::csr(0x344, "mip", false));

        // Machine protection and translation
        registers.push(RiscvRegister::csr(0x3a0, "mpmcfg0", false));
        registers.push(RiscvRegister::csr(0x3a1, "mpmcfg1", false));
        registers.push(RiscvRegister::csr(0x3a2, "mpmcfg2", false));
        registers.push(RiscvRegister::csr(0x3a3, "mpmcfg3", false));
        for pmpaddr_n in 0..16 {
            registers.push(RiscvRegister::csr(0x3b0 + pmpaddr_n, &format!("pmpaddr{}", pmpaddr_n), false));
        }

        // Machine counter/timers
        registers.push(RiscvRegister::csr(0xb00, "mcycle", false));
        registers.push(RiscvRegister::csr(0xb02, "minstret", false));
        for mhpmcounter_n in 3..32 {
            registers.push(RiscvRegister::csr(0xb00 + mhpmcounter_n, &format!("mhpmcounter{}", mhpmcounter_n), false));
        }
        registers.push(RiscvRegister::csr(0xb80, "mcycleh", false));
        registers.push(RiscvRegister::csr(0xb82, "minstreth", false));
        for mhpmcounter_n in 3..32 {
            registers.push(RiscvRegister::csr(0xb80 + mhpmcounter_n, &format!("mhpmcounter{}h", mhpmcounter_n), false));
        }

        // Machine counter setup
        for mhpmevent_n in 3..32 {
            registers.push(RiscvRegister::csr(0x320 + mhpmevent_n, &format!("mhpmevent{}", mhpmevent_n), false));
        }

        // Debug/trace registers
        registers.push(RiscvRegister::csr(0x7a0, "tselect", false));
        registers.push(RiscvRegister::csr(0x7a1, "tdata1", false));
        registers.push(RiscvRegister::csr(0x7a2, "tdata2", false));
        registers.push(RiscvRegister::csr(0x7a3, "tdata3", false));

        // Debug mode registers
        registers.push(RiscvRegister::csr(0x7b0, "dcsr", false));
        registers.push(RiscvRegister::csr(0x7b1, "dpc", false));
        registers.push(RiscvRegister::csr(0x7b2, "dscratch", false));

        registers
    }

    fn make_target_xml(registers: &Vec<RiscvRegister>) -> String {
        let mut target_xml = r#"
            <?xml version=\"1.0\"?>
                <!DOCTYPE target SYSTEM "gdb-target.dtd">
                <target version="1.0">
        "#.to_string();

        // Add in general-purpose registers
        for ft in &[RiscvRegisterType::General, RiscvRegisterType::CSR] {
            target_xml.push_str(&format!("<feature name=\"{}\">\n", ft.feature_name()));
            for reg in registers {
                if ! reg.present || reg.register_type != *ft {
                    continue;
                }
                target_xml.push_str(
                    &format!("<reg name=\"{}\" bitsize=\"32\" regnum=\"{}\" save-restore=\"no\" type=\"int\" group=\"{}\"/>\n",
                        reg.name, reg.index, reg.register_type.group())
                );
            }
        }
        target_xml.push_str("</feature>\n");
        target_xml.push_str("</target>\n");

        target_xml
    }

    pub fn get_feature(&self, name: &str) -> Result<Vec<u8>, RiscvCpuError> {
        if name == "target.xml" {
            let xml = self.target_xml.to_string().into_bytes();
            Ok(xml)
        } else {
            Err(RiscvCpuError::UnrecognizedFile(name.to_string()))
        }
    }

    pub fn get_threads(&self) -> Result<Vec<u8>, RiscvCpuError> {
        Ok(THREADS_XML.to_string().into_bytes())
    }
}