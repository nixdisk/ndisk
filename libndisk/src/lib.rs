use std::fmt::format;
use std::io::{Error, ErrorKind, Result, Seek, SeekFrom, Write};
use std::fs::File;
use std::path::Path;
use tempfile::NamedTempFile;

use ::libparted::Device;
use ::libparted::Disk;
use ::libparted::DiskType;
use ::libparted::Partition;
use ::libparted::PartitionType;

//TODO general patterns, these can be done in just about any combo/order
// - RAID creation
// - libparted functions (partitioning, some formatting)
// - luks functions
// - btrfs or zfs

//  simple tasks
//  1) create GUID partition map
//  2) create MBR partition map
//  3) create single partition
//  4) create multiple partitions
//  5) format partition as ext4 fs

// TODO keep a list of opened devices to ensure finish or abort is always called on them


const SUPPORTED_DISK_TYPES: [&str; 1] = ["guid"];

enum PartitionSizeType {
    EntireDisk,
    NextAvailablePlusOffsetBytes, // start the partition at the next available sector. add the
                                  // specified offset bytes to get the end of the partition
    NextAvailablePlusOffsetSectors, // start the partition at the next available sector. add the
                                    // specified offset sectors to get the end of the partition
    NextAvailablePlusOffsetPercentage, // start the partition at the next available sector.
                                       // calculate the number of offset bytes to add based on
                                       // percentage of disk size
    ExactSize, // specific start and end sector
}

pub struct PartitionSize<> {
    size_type: PartitionSizeType,
    start: i64,
    end: i64,
    offset: i64,
}

pub struct PartitionPlan<> {
    name: String,
    size: PartitionSize,
    part_flag: String, // set things like boot, esp, raid, etc
                       // https://www.gnu.org/software/parted/api/group__PedDisk.html#ga663201a9e2e2580a15579858944fddb7
}

pub struct LibpartedDevicePlan<> {
    partitions: Vec<PartitionPlan>,
    disk_type: String, // something from SUPPORTED_DISK_TYPES
}

pub fn construct_libparted_partition(disk: &Disk, part_plan: PartitionPlan) -> Result<Partition> {
// TODO
//     // TODO take type as an arg? idk if PartitionType is something users will need to set
//     // part types are talked about here https://www.gnu.org/software/parted/api/struct__PedPartition.html
//     // TODO take fs_type as an arg, let the user decide what type of fs

    let size = part_plan.size;
    if size.size_type == PartitionSizeType::EntireDisk {
        // TODO calculate the start and end usable space on the entire disk
        println!("TODO");
    }
    else if size.size_type == PartitionSizeType::ExactSize {
        let start = size.start;
        let end = size.end;
    }
    else if size.size_type == PartitionSizeType::NextAvailablePlusOffsetBytes {
        //TODO get the next available sector
        // then convert the bytes to sectors, rounding
        // add that to the found start to get the end
        println!("TODO");
    }
    else if size.size_type == PartitionSizeType::NextAvailablePlusOffsetPercentage{
        //TODO get the next available sector
        // then convert the percentage to sectors, rounding
        // add that to the found start to get the end
        println!("TODO");
    }
    else if size.size_type == PartitionSizeType::NextAvailablePlusOffsetSectors{
        //TODO get the next available sector
        // then add the offset to the found start to get the end
        println!("TODO");
    }

    return Partition::new(disk, PartitionType::PedPartitionNormal, None, start, end);
}

pub fn execute_libparted_device_plan(device_path: &Path, plan: &LibpartedDevicePlan) -> Result<()> {
    //TODO take an "ignore_existing" flag to force create of gpt part map
    let mut dev = Device::new(device_path)?;
    let optimal_constraint = dev.get_optimal_aligned_constraint()?;
    //TODO warn!(SUPPORTED_DISK_TYPES.contains(plan.disk_type), "unsupported disk type {plan.disk_type}");
    let disk_type = DiskType::get(plan.disk_type).ok_or(Error::new(ErrorKind::NotFound, "Invalid disk type supplied"))?;

    let disk = Disk::new_fresh(&mut dev, disk_type)?; // use new instead of new_fresh to read any existing partition map

    for part_plan in plan.partitions {
        let part = construct_libparted_partition(&disk, part_plan)?;
        //TODO allow user to override this? there may be times where we want to use
        // dev.get_constraint() instead to ensure control over the partition location?
        disk.add_partition(part, optimal_constraint);
    }
    disk.commit();
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;


    fn create_empty_file(path: &Path) {
        let mut file = File::create(path).unwrap();
        // 50MiB
        file.seek(SeekFrom::Start(52428800)).unwrap();
        file.write_all(&[0]).unwrap();
    }

    fn create_test_libparted_plan() -> LibpartedDevicePlan {
        let partition_size = PartitionSize; //TODO test different sizes
        partition_size.size_type = PartitionSizeType::ExactSize;
        partition_size.start = 10;
        partition_size.end = 30;

        let partition_plan = PartitionPlan;
        partition_plan.name = "root";
        partition_plan.size = partition_size;

        let plans = vec![partition_plan];

        let libparted_device_plan = LibpartedDevicePlan;
        libparted_device_plan.disk_type = "guid";
        libparted_device_plan.partitions = plans;

        return libparted_device_plan;
    }

    #[test]
    //TODO automatically test all partition types, with a bunch of different sizes, filesystems,
    //etc?
    fn test_create_single_gpt_partition(){
        // create a temp device with an empty gpt partition table
        let tmp_dev_file = NamedTempFile::new().expect("unable to get a tempfile to test with");
        let tmp_dev_path = tmp_dev_file.path();
        create_empty_file(tmp_dev_path);

        let plans = create_test_libparted_plan();
        assert!(execute_libparted_device_plan(tmp_dev_path, plans), "failed to execute libparted device plan");

        // confirm the partition table exists and is empty

        // let mut test_dev = Device::new(tmp_dev_path).expect("unable to create a test device from the tmp_dev_path");
        // let mut test_disk = Disk::new(&mut test_dev).expect("unable to create a test_disk from the test_dev");
        // assert!(test_disk.check().is_err(), "test_disk does not have a proper gpt partition table");
        // let test_parts = test_disk.parts();
        // assert!(test_parts.count() == 0, "test_disk with empty gpt partition table should not have any partitions");
    }
}
