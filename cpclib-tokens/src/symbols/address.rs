use std::fmt::Display;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAddress {
    Memory(MemoryPhysicalAddress),
    Bank(BankPhysicalAddress),
    Cpr(CprPhysicalAddress)
}

impl From<u16> for PhysicalAddress {
    fn from(value: u16) -> Self {
        Self::Memory(MemoryPhysicalAddress::new(value, 0xC0))
    }
}
impl Display for PhysicalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicalAddress::Memory(address) => {
                write!(
                    f,
                    "0x{:X} (0x{:X} in page {})",
                    address.address(),
                    address.offset_in_page(),
                    address.page(),
                )
            },
            PhysicalAddress::Cpr(address) => {
                write!(
                    f,
                    "0x{:X} in Cartridge bloc {}",
                    address.address(),
                    address.bloc()
                )
            },
            PhysicalAddress::Bank(address) => {
                write!(f, "0x{:X} in bank {}", address.address(), address.bank())
            }
        }
    }
}

impl PhysicalAddress {
    #[inline(always)]
    pub fn address(&self) -> u16 {
        match self {
            PhysicalAddress::Memory(adr) => adr.address(),
            PhysicalAddress::Bank(adr) => adr.address(),
            PhysicalAddress::Cpr(adr) => adr.address()
        }
    }

    /// not really coherent to use that with cpr and bank
    #[inline(always)]
    pub fn offset_in_cpc(&self) -> u32 {
        match self {
            PhysicalAddress::Memory(adr) => adr.offset_in_cpc(),
            PhysicalAddress::Bank(adr) => adr.address() as _,
            PhysicalAddress::Cpr(adr) => adr.address() as _
        }
    }

    #[inline(always)]
    pub fn to_memory(self) -> MemoryPhysicalAddress {
        match self {
            PhysicalAddress::Memory(adr) => adr,
            _ => panic!()
        }
    }

    #[inline(always)]
    pub fn to_bank(self) -> BankPhysicalAddress {
        match self {
            PhysicalAddress::Bank(adr) => adr,
            _ => panic!()
        }
    }

    #[inline(always)]
    pub fn to_cpr(self) -> CprPhysicalAddress {
        match self {
            PhysicalAddress::Cpr(adr) => adr,
            _ => panic!()
        }
    }

    pub fn remu_bank(&self) -> u16 {
        match self {
            PhysicalAddress::Memory(m) => (4 * m.page as u16 + (m.address / 0x4000)) as _,
            PhysicalAddress::Bank(b) => b.bank() as _,
            PhysicalAddress::Cpr(c) => c.bloc() as _
        }
    }
}

impl From<MemoryPhysicalAddress> for PhysicalAddress {
    #[inline(always)]
    fn from(value: MemoryPhysicalAddress) -> Self {
        Self::Memory(value)
    }
}

impl From<BankPhysicalAddress> for PhysicalAddress {
    #[inline(always)]
    fn from(value: BankPhysicalAddress) -> Self {
        Self::Bank(value)
    }
}

impl From<CprPhysicalAddress> for PhysicalAddress {
    #[inline(always)]
    fn from(value: CprPhysicalAddress) -> Self {
        Self::Cpr(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CprPhysicalAddress {
    bloc: u8,
    address: u16
}

impl CprPhysicalAddress {
    #[inline]
    pub fn new(address: u16, bloc: u8) -> Self {
        Self { bloc, address }
    }

    #[inline]
    pub fn address(&self) -> u16 {
        self.address
    }

    #[inline]
    pub fn bloc(&self) -> u8 {
        self.bloc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankPhysicalAddress {
    bank: usize,
    address: u16
}

impl BankPhysicalAddress {
    #[inline]
    pub fn new(address: u16, bank: usize) -> Self {
        Self { bank, address }
    }

    #[inline]
    pub fn address(&self) -> u16 {
        self.address
    }

    #[inline]
    pub fn bank(&self) -> usize {
        self.bank
    }
}

/// Structure that ease the addresses manipulation to read/write at the right place
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPhysicalAddress {
    /// Page number (0 for base, 1 for first page, 2 ...)
    page: u8,
    /// Bank number in the page: 0 to 3
    bank: u8,
    /// Address manipulate by CPU 0x0000 to 0xffff
    address: u16
}

impl From<u16> for MemoryPhysicalAddress {
    fn from(nb: u16) -> Self {
        MemoryPhysicalAddress::new(nb, 0xC0)
    }
}

impl MemoryPhysicalAddress {
    pub fn new(address: u16, mmr: u8) -> Self {
        if mmr == 0xC1 {
            return MemoryPhysicalAddress {
                page: 1,
                bank: (address / 0x4000) as u8,
                address: address % 0x4000
            };
        }

        let possible_page = ((mmr >> 3) & 0b111) + 1;
        let possible_bank = mmr & 0b11;
        let standard_bank = match address {
            0x0000..0x4000 => 0,
            0x4000..0x8000 => 1,
            0x8000..0xC000 => 2,
            0xC000.. => 3
        };
        let is_4000 = (0x4000..0x8000).contains(&address);
        let is_c000 = address >= 0xC000;

        let (page, bank) = if (mmr & 0b100) != 0 {
            if is_4000 {
                (possible_page, possible_bank)
            }
            else {
                (0, possible_bank)
            }
        }
        else {
            match mmr & 0b11 {
                0b000 => (0, standard_bank),
                0b001 => {
                    if is_c000 {
                        (possible_page, standard_bank)
                    }
                    else {
                        (0, standard_bank)
                    }
                },
                0b010 => (possible_page, standard_bank),
                0b011 => {
                    if is_4000 {
                        (0, 3)
                    }
                    else if is_c000 {
                        (possible_page, 3)
                    }
                    else {
                        (0, standard_bank)
                    }
                },
                _ => unreachable!()
            }
        };

        Self {
            address,
            bank,
            page
        }
    }

    pub fn offset_in_bank(&self) -> u16 {
        self.address % 0x4000
    }

    pub fn offset_in_page(&self) -> u16 {
        self.offset_in_bank() + self.bank as u16 * 0x4000
    }

    pub fn offset_in_cpc(&self) -> u32 {
        self.offset_in_page() as u32 + self.page as u32 * 0x1_0000
    }

    pub fn address(&self) -> u16 {
        self.address
    }

    pub fn bank(&self) -> u8 {
        self.bank
    }

    pub fn page(&self) -> u8 {
        self.page
    }

    pub fn ga_bank(&self) -> u16 {
        let low = if self.page() == 0 {
            0b1100_0000
        }
        else {
            0b1100_0100 + ((self.page() - 1) << 3) + self.bank
        } as u16;
        low + 0x7F00
    }

    pub fn ga_page(&self) -> u16 {
        let low = if self.page() == 0 {
            0b1100_0000
        }
        else {
            0b1100_0010 + ((self.page() - 1) << 3)
        } as u16;
        low + 0x7F00
    }
}
