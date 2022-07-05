    ($($v:expr),*) => {
        Vec3::from(($($v),*))
    };
}
macro_rules! vec4 {
    ($($v:expr),*) => {
        Vec4::from(($($v),*))
    };
}

macro_rules! ivec2 {
    ($($v:expr),*) => {
        IVec2::from(($($v),*))
    };
}
macro_rules! ivec3 {
    ($($v:expr),*) => {
        IVec3::from(($($v),*))
    };
}
macro_rules! ivec4 {
    ($($v:expr),*) => {
        IVec4::from(($($v),*))
    };
}

macro_rules! uvec2 {
    ($($v:expr),*) => {
        UVec2::from(($($v),*))
    };
}
macro_rules! uvec3 {
    ($($v:expr),*) => {
        UVec3::from(($($v),*))
    };
}
macro_rules! uvec4 {
    ($($v:expr),*) => {
        UVec4::from(($($v),*))
    };
}

macro_rules! mat2{
    ($($v:expr),*) => {
        Mat2::from_cols($($v),*)
    }
}
macro_rules! mat3{
    ($($v:expr),*) => {
        Mat3::from_cols($($v),*)
    }
}
macro_rules! mat4{
    ($($v:expr),*) => {
        Mat4::from_cols($($v),*)
    }
}
