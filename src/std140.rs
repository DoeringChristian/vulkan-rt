pub trait UniformBlock {
    fn write_to(&self, dst: &mut impl std::io::Write);
}

pub trait Member {
    const ALLIGNMENT: usize;
}

impl Member for u32 {
    const ALLIGNMENT: usize = 4;
}
impl Member for [u32; 2] {
    const ALLIGNMENT: usize = 8;
}
impl Member for [u32; 4] {
    const ALLIGNMENT: usize = 16;
}
impl Member for f32 {
    const ALLIGNMENT: usize = 4;
}
impl Member for [f32; 2] {
    const ALLIGNMENT: usize = 8;
}
impl Member for [f32; 4] {
    const ALLIGNMENT: usize = 16;
}

#[cfg(test)]
mod test {
    use super::Member;
    use super::UniformBlock;

    pub struct TestUB {
        a: f32,
        b: [f32; 4],
        c: Vec<f32>,
    }
    impl UniformBlock for TestUB {
        fn write_to(&self, dst: &mut impl std::io::Write) {
            let mut count = 0;
            count += dst.write(bytemuck::cast_slice(&[self.a])).unwrap();

            while count % <[f32; 4]>::ALLIGNMENT != 0 {
                count += dst.write(bytemuck::cast_slice(&[0])).unwrap();
            }
        }
    }
    #[test]
    fn test() {}
}
