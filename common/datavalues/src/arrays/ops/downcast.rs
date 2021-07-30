// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use std::sync::Arc;

use common_arrow::arrow::array::*;

use crate::arrays::DataArray;
use crate::prelude::*;
use crate::series::IntoSeries;
use crate::series::Series;
use crate::DFBinaryArray;
use crate::DFBooleanArray;
use crate::DFListArray;
use crate::DFPrimitiveType;
use crate::DFStructArray;
use crate::DFUtf8Array;

impl<T> AsRef<PrimitiveArray<T::Native>> for DataArray<T>
where T: DFPrimitiveType
{
    fn as_ref(&self) -> &PrimitiveArray<T::Native> {
        self.downcast_ref()
    }
}

impl<T> DataArray<T>
where T: DFPrimitiveType
{
    pub fn downcast_ref(&self) -> &PrimitiveArray<T::Native> {
        let arr = &*self.array;
        unsafe { &*(arr as *const dyn Array as *const PrimitiveArray<T::Native>) }
    }

    pub fn downcast_iter(&self) -> impl Iterator<Item = Option<T::Native>> + DoubleEndedIterator {
        let arr = &*self.array;
        let arr = unsafe { &*(arr as *const dyn Array as *const PrimitiveArray<T::Native>) };
        arr.iter()
    }

    pub fn collect_values(&self) -> Vec<Option<T::Native>> {
        self.downcast_iter().collect()
    }

    pub fn from_arrow_array(array: PrimitiveArray<T::Native>) -> Self {
        let array_ref = Arc::new(array) as ArrayRef;
        array_ref.into()
    }
}

impl AsRef<BooleanArray> for DFBooleanArray {
    fn as_ref(&self) -> &BooleanArray {
        self.downcast_ref()
    }
}

impl DFBooleanArray {
    pub fn downcast_ref(&self) -> &BooleanArray {
        let arr = &*self.array;
        unsafe { &*(arr as *const dyn Array as *const BooleanArray) }
    }

    pub fn downcast_iter(&self) -> impl Iterator<Item = Option<bool>> + DoubleEndedIterator {
        let arr = &*self.array;
        let arr = unsafe { &*(arr as *const dyn Array as *const BooleanArray) };
        arr.iter()
    }

    pub fn collect_values(&self) -> Vec<Option<bool>> {
        self.downcast_iter().collect()
    }

    pub fn from_arrow_array(array: BooleanArray) -> Self {
        let array_ref = Arc::new(array) as ArrayRef;
        array_ref.into()
    }
}

impl AsRef<LargeUtf8Array> for DFUtf8Array {
    fn as_ref(&self) -> &LargeUtf8Array {
        self.downcast_ref()
    }
}

impl DFUtf8Array {
    pub fn downcast_ref(&self) -> &LargeUtf8Array {
        let arr = &*self.array;
        unsafe { &*(arr as *const dyn Array as *const LargeUtf8Array) }
    }

    pub fn downcast_iter<'a>(&self) -> impl Iterator<Item = Option<&'a str>> + DoubleEndedIterator {
        let arr = &*self.array;
        let arr = unsafe { &*(arr as *const dyn Array as *const LargeUtf8Array) };
        arr.iter()
    }

    pub fn collect_values<'a>(&self) -> Vec<Option<&'a str>> {
        self.downcast_iter().collect()
    }

    pub fn from_arrow_array(array: LargeUtf8Array) -> Self {
        let array_ref = Arc::new(array) as ArrayRef;
        array_ref.into()
    }
}

impl AsRef<LargeListArray> for DFListArray {
    fn as_ref(&self) -> &LargeListArray {
        self.downcast_ref()
    }
}

impl DFListArray {
    pub fn downcast_ref(&self) -> &LargeListArray {
        let arr = &*self.array;
        unsafe { &*(arr as *const dyn Array as *const LargeListArray) }
    }

    pub fn downcast_iter(&self) -> impl Iterator<Item = Option<Series>> + DoubleEndedIterator {
        let arr = &*self.array;
        let arr = unsafe { &*(arr as *const dyn Array as *const LargeListArray) };

        arr.iter().map(|a| a.map(|a| a.into_series()))
    }

    pub fn from_arrow_array(array: LargeListArray) -> Self {
        let array_ref = Arc::new(array) as ArrayRef;
        array_ref.into()
    }
}

impl AsRef<LargeBinaryArray> for DFBinaryArray {
    fn as_ref(&self) -> &LargeBinaryArray {
        self.downcast_ref()
    }
}

impl DFBinaryArray {
    pub fn downcast_ref(&self) -> &LargeBinaryArray {
        let arr = &*self.array;
        unsafe { &*(arr as *const dyn Array as *const LargeBinaryArray) }
    }

    pub fn from_arrow_array(array: LargeBinaryArray) -> Self {
        let array_ref = Arc::new(array) as ArrayRef;
        array_ref.into()
    }
}

impl AsRef<StructArray> for DFStructArray {
    fn as_ref(&self) -> &StructArray {
        self.downcast_ref()
    }
}
impl DFStructArray {
    pub fn downcast_ref(&self) -> &StructArray {
        let arr = &*self.array;
        unsafe { &*(arr as *const dyn Array as *const StructArray) }
    }

    pub fn from_arrow_array(array: StructArray) -> Self {
        let array_ref = Arc::new(array) as ArrayRef;
        array_ref.into()
    }
}
