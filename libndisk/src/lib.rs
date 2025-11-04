use std::fmt;
use std::fmt::format;
use std::fs::File;
use std::io::{Error, ErrorKind, Result, Seek, SeekFrom, Write};
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

const SUPPORTED_DISK_TYPES: [&str; 1] = ["gpt"];

trait LibPartedPartition {
    fn to_string(&self) -> String;
}

impl LibPartedPartition for Partition<'_> {
    fn to_string(&self) -> String {
        return format!(
            "partition name: {:?}, num: {}, start: {}, end: {}, type: {}",
            self.name(),
            self.num(),
            self.geom_start(),
            self.geom_end(),
            self.type_get_name()
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
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

pub struct PartitionSize {
    size_type: PartitionSizeType,
    start: Option<i64>,
    end: Option<i64>,
    offset: Option<i64>,
}

impl PartitionSize {
    fn new(
        size_type: PartitionSizeType,
        start: Option<i64>,
        end: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PartitionSize> {
        if size_type == PartitionSizeType::EntireDisk {
            return Ok(PartitionSize {
                size_type: size_type,
                start: None,
                end: None,
                offset: None,
            });
        } else if [
            PartitionSizeType::NextAvailablePlusOffsetSectors,
            PartitionSizeType::NextAvailablePlusOffsetPercentage,
            PartitionSizeType::NextAvailablePlusOffsetBytes,
        ]
        .contains(&size_type)
        {
            let valid_offset = offset.ok_or(Error::new(
                ErrorKind::NotFound,
                "size type requires an offset",
            ))?;
            return Ok(PartitionSize {
                size_type: size_type,
                start: None,
                end: None,
                offset: Some(valid_offset),
            });
        } else if size_type == PartitionSizeType::ExactSize {
            let valid_start = start.ok_or(Error::new(
                ErrorKind::NotFound,
                "ExactSize size type requires a start",
            ))?;
            let valid_end = start.ok_or(Error::new(
                ErrorKind::NotFound,
                "ExactSize size type requires a end",
            ))?;
            return Ok(PartitionSize {
                size_type: size_type,
                start: Some(valid_start),
                end: Some(valid_end),
                offset: None,
            });
        } else {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("unsupported PartitionSizeType: {size_type:#?}"),
            ));
        }
    }
}

pub struct PartitionPlan {
    name: String,
    size: PartitionSize,
    part_flag: Option<String>, // set things like boot, esp, raid, etc
                               // https://www.gnu.org/software/parted/api/group__PedDisk.html#ga663201a9e2e2580a15579858944fddb7
}

pub struct LibpartedDevicePlan {
    partitions: Vec<PartitionPlan>,
    disk_type: String, // something from SUPPORTED_DISK_TYPES
}

fn get_next_free_partition<'a>(disk: &'a Disk) -> Result<Partition<'a>> {
    // locate the free partition
    // any newly created gpt has 3 partitions
    // metadata at the start
    // free space partition
    // metadata at the end
    for part in disk.parts() {
        println!("{}", part.to_string());
        if part.type_get_name() == "free" {
            println!("next free partition is: {}", part.to_string());
            return Ok(part);
        }
    }

    return Err(Error::new(
        ErrorKind::NotFound,
        "disk does not have any free space",
    ));
}

fn contains_nonfree_partitions(disk: &Disk) -> bool {
    for part in disk.parts() {
        println!("{}", part.to_string());
        if !["free", "metadata"].contains(&part.type_get_name()) {
            return true;
        }
    }
    return false;
}

pub fn construct_libparted_partition<'a>(
    disk: &Disk,
    part_plan: &PartitionPlan,
) -> Result<Partition<'a>> {
    // TODO
    //     // TODO take type as an arg? idk if PartitionType is something users will need to set
    //     // part types are talked about here https://www.gnu.org/software/parted/api/struct__PedPartition.html
    //     // TODO take fs_type as an arg, let the user decide what type of fs

    let size = &part_plan.size;
    let start;
    let end;

    let next_free_partition = get_next_free_partition(&disk)?;

    if size.size_type == PartitionSizeType::EntireDisk {
        // this type is invalid if there is already another partition on the disk
        // please wipe the disk and create a new partition table before calling this function
        if contains_nonfree_partitions(&disk) {
            return Err(Error::new(
                ErrorKind::NotFound,
                "disk already contains partitions, unable to use whole disk",
            ));
        }
        start = next_free_partition.geom_start();
        end = next_free_partition.geom_end();
    } else if size.size_type == PartitionSizeType::ExactSize {
        start = size.start.unwrap();
        end = size.end.unwrap();
    }
    // else if size.size_type == PartitionSizeType::NextAvailablePlusOffsetBytes {
    //     //TODO get the next available sector
    //     // then convert the bytes to sectors, rounding
    //     // add that to the found start to get the end
    //     println!("TODO");
    // }
    // else if size.size_type == PartitionSizeType::NextAvailablePlusOffsetPercentage{
    //     //TODO NEED THE TOTAL DISK SIZE TO CALCULATE THIS, PERHAPS ADD TOGETHER ALL NON METADATA
    //     PARTITIONS LENGTHS??
    //     //TODO get the next available sector
    //     // then convert the percentage to sectors, rounding
    //     // add that to the found start to get the end
    //     println!("TODO");
    // }
    // else if size.size_type == PartitionSizeType::NextAvailablePlusOffsetSectors{
    //     //TODO get the next available sector
    //     // then add the offset to the found start to get the end
    //     println!("TODO");
    // }
    else {
        return Err(Error::new(ErrorKind::NotFound, "unknown PartitionSizeType"));
    }

    return Partition::new(disk, PartitionType::PED_PARTITION_NORMAL, None, start, end);
}

pub fn execute_libparted_device_plan(device_path: &Path, plan: &LibpartedDevicePlan) -> Result<()> {
    //TODO take an "ignore_existing" flag to force create of gpt part map
    let mut dev = Device::new(device_path)?;
    let optimal_constraint = dev.get_optimal_aligned_constraint()?;
    //TODO warn!(SUPPORTED_DISK_TYPES.contains(&plan.disk_type), "unsupported disk type {plan.disk_type}");
    let disk_type = DiskType::get(&plan.disk_type).ok_or(Error::new(
        ErrorKind::NotFound,
        "Invalid disk type supplied",
    ))?;

    let mut disk = Disk::new_fresh(&mut dev, disk_type)?; // use new instead of new_fresh to read any existing partition map

    println!("creating {} partitions", plan.partitions.len());

    for part_plan in &plan.partitions {
        let mut part = construct_libparted_partition(&mut disk, part_plan)?;
        //TODO allow user to override this? there may be times where we want to use
        // dev.get_constraint() instead to ensure control over the partition location?
        disk.add_partition(&mut part, &optimal_constraint)?;
    }
    disk.commit()?;
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
        let partition_size =
            PartitionSize::new(PartitionSizeType::ExactSize, Some(10), Some(30), None)
                .expect("failed to create test libparted plan partition size");
        let partition_plan = PartitionPlan {
            name: "root".to_string(),
            size: partition_size,
            part_flag: None,
        };
        let libparted_device_plan = LibpartedDevicePlan {
            disk_type: "gpt".to_string(),
            partitions: vec![partition_plan],
        };

        return libparted_device_plan;
    }

    //TODO automatically test all partition types, with a bunch of different sizes, filesystems,
    //etc?
    #[test]
    fn test_fresh_gpt_partition_map() -> Result<()> {
        // create a temp device with an empty gpt partition table
        let tmp_dev_file = NamedTempFile::new().expect("unable to get a tempfile to test with");
        let tmp_dev_path = tmp_dev_file.path();
        create_empty_file(tmp_dev_path);

        let libparted_device_plan = LibpartedDevicePlan {
            disk_type: "gpt".to_string(),
            partitions: vec![],
        };

        execute_libparted_device_plan(&tmp_dev_path, &libparted_device_plan)?;

        // confirm the partition table exists and is empty

        let mut test_dev = Device::new(tmp_dev_path)
            .expect("unable to create a test device from the tmp_dev_path");
        let mut test_disk =
            Disk::new(&mut test_dev).expect("unable to create a test_disk from the test_dev");
        assert!(
            test_disk.check().is_ok(),
            "test_disk does not have a proper gpt partition table"
        );
        let test_parts = test_disk.parts();
        assert!(
            test_disk.parts().count() == 3,
            "test_disk with empty gpt partition table should only have 3 partitions, 2 metadata and 1 free"
        );
        //TODO check for the 2 metadata and 1 free partition explicitly by partition id type name
        return Ok(());
    }
    #[test]
    fn test_create_single_gpt_partition() -> Result<()> {
        // create a temp device with an empty gpt partition table
        let tmp_dev_file = NamedTempFile::new().expect("unable to get a tempfile to test with");
        let tmp_dev_path = tmp_dev_file.path();
        create_empty_file(tmp_dev_path);

        let plans = create_test_libparted_plan();

        return execute_libparted_device_plan(&tmp_dev_path, &plans);

        // confirm the partition table exists and is empty

        // let mut test_dev = Device::new(tmp_dev_path).expect("unable to create a test device from the tmp_dev_path");
        // let mut test_disk = Disk::new(&mut test_dev).expect("unable to create a test_disk from the test_dev");
        // assert!(test_disk.check().is_ok(), "test_disk does not have a proper gpt partition table");
        // let test_parts = test_disk.parts();
        // assert!(test_parts.count() == 0, "test_disk with empty gpt partition table should not have any partitions");
    }
}
