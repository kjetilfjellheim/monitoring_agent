use std::{collections::HashMap, sync::{Arc, Mutex}};

use log::{error, info};
use monitoring_agent_lib::proc::ProcsLoadavg;
use tokio_cron_scheduler::Job;

use crate::{common::{configuration::DatabaseStoreLevel, ApplicationError, MonitorStatus, Status}, MariaDbService};

use super::Monitor;

#[derive(Debug, Clone)]
pub struct LoadAvgMonitor {
    /// The name of the monitor.
    pub name: String,
    /// Max load average for 1 minute.
    pub loadavg1min_max: Option<f32>,
    /// Max load average for 5 minutes.
    pub loadavg5min_max: Option<f32>,
    /// Max load average for 10 minutes.
    pub loadavg10min_max: Option<f32>,
    /// The status of the monitor.
    pub status: Arc<Mutex<HashMap<String, MonitorStatus>>>,
    /// The database service.
    database_service: Arc<Option<MariaDbService>>,
    /// The database store level.
    database_store_level: DatabaseStoreLevel,
    /// The current load average.
    store_current_loadavg: bool,              
}

impl LoadAvgMonitor {

    /**
     * Create a new load average monitor.
     * 
     * `name`: The name of the monitor.
     * `loadavg1min_max`: The max load average for 1 minute.
     * `loadavg5min_max`: The max load average for 5 minutes.
     * `loadavg10min_max`: The max load average for 10 minutes.
     * `status`: The status of the monitor.
     * `database_service`: The database service.
     * `database_store_level`: The database store level.
     * `store_current_loadavg`: Store the current load average.
     * 
     * Returns: A new load average monitor.
     * 
     */
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::similar_names)]    
    pub fn new(
        name: &str,
        loadavg1min_max: Option<f32>,
        loadavg5min_max: Option<f32>,
        loadavg10min_max: Option<f32>,
        status: &Arc<Mutex<HashMap<String, MonitorStatus>>>,
        database_service: &Arc<Option<MariaDbService>>,
        database_store_level: &DatabaseStoreLevel,
        store_current_loadavg: bool,
    ) -> LoadAvgMonitor {

        let status_lock = status.lock();
        match status_lock {
            Ok(mut lock) => {
                lock.insert(name.to_string(), MonitorStatus::new(name.to_string(), Status::Unknown));
            }
            Err(err) => {
                error!("Error creating loadavg monitor: {:?}", err);
            }
        }

        LoadAvgMonitor {
            name: name.to_string(),
            loadavg1min_max,
            loadavg5min_max,
            loadavg10min_max,
            status: status.clone(),
            database_service: database_service.clone(),
            database_store_level: database_store_level.clone(),
            store_current_loadavg,
        }
    }

    /**
     * Check the load average.
     * 
     * `loadavg`: The current load average.
     * 
     */
    #[allow(clippy::similar_names)]         
    fn check_loadavg(&mut self, loadavg: &ProcsLoadavg) {    
        let status_1min = LoadAvgMonitor::check_loadavg_values(self.loadavg1min_max, loadavg.loadavg1min);
        let status_5min = LoadAvgMonitor::check_loadavg_values(self.loadavg5min_max, loadavg.loadavg5min);
        let status_10min = LoadAvgMonitor::check_loadavg_values(self.loadavg10min_max, loadavg.loadavg10min);
        
        if status_1min != Status::Ok || status_5min != Status::Ok || status_10min != Status::Ok {
            self.set_status(&Status::Error {
                message: format!(
                    "Load average check failed: 1min: {status_1min:?}, 5min: {status_5min:?}, 10min: {status_10min:?}"
                ),
            });
        } else {
            self.set_status(&Status::Ok);
        }
    }

    /**
     * Check the load average values.
     * 
     * `max`: The max load average.
     * `current`: The current load average.
     * 
     * Returns: The status of the check.
     * 
     */
    fn check_loadavg_values(max: Option<f32>, current: Option<f32>) -> Status {
        let Some(current) = current else { return Status::Ok };
        let Some(max) = max else { return Status::Ok };
            
        if current > max {
            return Status::Error {
                message: format!(
                    "Load average {current} is greater than max load average {max}"
                ),
            };
        }
        Status::Ok       
    }

    /**
     * Check and store the current load average.
     * 
     * `loadavg`: The current load average.
     * 
     * 
     */
    fn check_store_current_loadavg(&self, loadavg: &ProcsLoadavg) {
        if self.store_current_loadavg {
            self.store_current_loadavg(loadavg);
        }           
    }
    
    /**
     * Store the current load average.
     * 
     * `loadavg`: The current load average.
     */
    fn store_current_loadavg(&self, loadavg: &ProcsLoadavg) {
        match self.database_service.as_ref() {            
            Some(database_service) => {
                match database_service.store_loadavg(loadavg) {
                    Ok(()) => {}
                    Err(err) => {
                        error!("Error storing load average: {:?}", err);
                    }
                }
            }
            None => {}
        }        
    }

    /**
     * Get a loadavg monitor job.
     * 
     * `schedule`: The schedule.
     * `name`: The name of the monitor.
     * `threshold_1min`: The threshold for the 1 minute load average.
     * `threshold_5min`: The threshold for the 5 minute load average.
     * `threshold_10min`: The threshold for the 10 minute load average.
     * `store_values`: Store the values in the database.
     * `status`: The status.
     * `database_store_level`: The database store level.
     * 
     * `result`: The result of getting the loadavg monitor job.
     * 
     * throws: `ApplicationError`: If the job fails to be created.
     * 
     */
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::similar_names)]    
    pub fn get_loadavg_monitor_job(
        &mut self,
        schedule: &str,
    ) -> Result<Job, ApplicationError> {
        info!("Creating Tcp monitor: {}", &self.name);
        let mut loadavg_monitor = self.clone();       
        let job_result = Job::new(schedule, move |_uuid, _locked| {                
            loadavg_monitor.check();
        });        
        match job_result {
            Ok(job) => Ok(job),
            Err(err) => Err(ApplicationError::new(
                format!("Could not create job: {err}").as_str(),
            )), 
        }
    }    

    /**
     * Check the monitor.
     */
    fn check(&mut self) {
        let loadavg = ProcsLoadavg::get_loadavg();
        match loadavg {
            Ok(loadavg) => {
                self.check_store_current_loadavg(&loadavg);
                self.check_loadavg(&loadavg);
            }
            Err(err) => {
                error!("Error getting load average: {:?}", err);
            }
        }                    
    }    

}

/**
 * Implement the `Monitor` trait for `LoadAvgMonitor`.
 */
impl super::Monitor for LoadAvgMonitor {
    /**
     * Get the name of the monitor.
     *
     * Returns: The name of the monitor.
     */
    fn get_name(&self) -> &str {
        &self.name
    }

    /**
     * Get the status of the monitor.
     *
     * Returns: The status of the monitor.
     */
    fn get_status(&self) -> Arc<Mutex<HashMap<String, MonitorStatus>>> {
        self.status.clone()
    }

    /**
     * Get the database service.
     *
     * Returns: The database service.
     */
    fn get_database_service(&self) -> Arc<Option<MariaDbService>> {
        self.database_service.clone()
    }

    /**
     * Get the database store level.
     *
     * Returns: The database store level.
     */
    fn get_database_store_level(&self) -> DatabaseStoreLevel {
        self.database_store_level.clone()
    }
     
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::{Arc, Mutex}};
    use crate::services::monitors::LoadAvgMonitor;

    use super::Monitor;

    /**
     * Test the check_loadavg_values function.'
     * 
     * Test the following scenarios:
     * - Load average is equal to max load average.
     * - Load average is higher than max load average.
     * - Load average is max has value, but none is retrieved.
     * - Load average is max is None, but load average has value.
     * - Load average and max load average are None.
     */
    #[test]
    fn test_check_loadavg_values() {
        let status = LoadAvgMonitor::check_loadavg_values(Some(1.0), Some(1.0));
        assert_eq!(status, super::Status::Ok);

        let status = LoadAvgMonitor::check_loadavg_values(Some(1.0), Some(2.0));
        assert_eq!(status, super::Status::Error {
            message: "Load average 2 is greater than max load average 1".to_string()
        });

        let status: crate::common::Status = LoadAvgMonitor::check_loadavg_values(Some(1.0), None);
        assert_eq!(status, super::Status::Ok);

        let status = LoadAvgMonitor::check_loadavg_values(None, Some(1.0));
        assert_eq!(status, super::Status::Ok);

        let status = LoadAvgMonitor::check_loadavg_values(None, None);
        assert_eq!(status, super::Status::Ok);
    }

    /**
     * Test the check_loadavg function.
     * 
     * Test the following scenarios:
     * - Load average is lower on all.
     */
    #[test]
    fn test_check_loadavg_lower_on_all() {
        // Test success. Loadaverage lower on all
        let mut monitor = super::LoadAvgMonitor::new(
            "test",
            Some(1.0),
            Some(2.0),
            Some(3.0),
            &Arc::new(Mutex::new(HashMap::new())),
            &Arc::new(None),
            &super::DatabaseStoreLevel::None,
            false,
        );

        let loadavg = monitoring_agent_lib::proc::ProcsLoadavg {
            loadavg1min: Some(1.0),
            loadavg5min: Some(2.0),
            loadavg10min: Some(3.0),
            current_running_processes: Some(1),
            total_number_of_processes: Some(10)
        };

        monitor.check_loadavg(&loadavg);

        let status = monitor.get_status();
        let status = status.lock().unwrap();
        assert_eq!(status.get("test").unwrap().status, super::Status::Ok);
    }

    /**
     * Test the check_loadavg function.
     * 
     * Test the following scenarios:
     * - Load average is higher on 1 minute.
     */
    #[test]
    fn test_check_loadavg_1min_higher() {
        // Test success. Loadaverage lower on all
        let mut monitor = super::LoadAvgMonitor::new(
            "test",
            Some(1.0),
            Some(2.0),
            Some(3.0),
            &Arc::new(Mutex::new(HashMap::new())),
            &Arc::new(None),
            &super::DatabaseStoreLevel::None,
            false,
        );

        let loadavg = monitoring_agent_lib::proc::ProcsLoadavg {
            loadavg1min: Some(1.1),
            loadavg5min: Some(2.0),
            loadavg10min: Some(3.0),
            current_running_processes: Some(1),
            total_number_of_processes: Some(10)
        };

        monitor.check_loadavg(&loadavg);

        let status = monitor.get_status();
        let status = status.lock().unwrap();
        assert_eq!(status.get("test").unwrap().status, super::Status::Error { message: "Load average check failed: 1min: Error { message: \"Load average 1.1 is greater than max load average 1\" }, 5min: Ok, 10min: Ok".to_string() } );
    }

    /**
     * Test the check_loadavg function.
     * 
     * Test the following scenarios:
     * - Load average is higher on 5 minutes.
     */
    #[test]
    fn test_check_loadavg_5min_higher() {
        // Test success. Loadaverage lower on all
        let mut monitor = super::LoadAvgMonitor::new(
            "test",
            Some(1.0),
            Some(2.0),
            Some(3.0),
            &Arc::new(Mutex::new(HashMap::new())),
            &Arc::new(None),
            &super::DatabaseStoreLevel::None,
            false,
        );

        let loadavg = monitoring_agent_lib::proc::ProcsLoadavg {
            loadavg1min: Some(1.0),
            loadavg5min: Some(2.1),
            loadavg10min: Some(3.0),
            current_running_processes: Some(1),
            total_number_of_processes: Some(10)
        };

        monitor.check_loadavg(&loadavg);

        let status = monitor.get_status();
        let status = status.lock().unwrap();
        assert_eq!(status.get("test").unwrap().status, super::Status::Error { message: "Load average check failed: 1min: Ok, 5min: Error { message: \"Load average 2.1 is greater than max load average 2\" }, 10min: Ok".to_string() } );
    }

    /**
     * Test the check_loadavg function.
     * 
     * Test the following scenarios:
     * - Load average is higher on 10 minutes.
     */ 
    #[test]
    fn test_check_loadavg_10min_higher() {
        // Test success. Loadaverage lower on all
        let mut monitor = super::LoadAvgMonitor::new(
            "test",
            Some(1.0),
            Some(2.0),
            Some(3.0),
            &Arc::new(Mutex::new(HashMap::new())),
            &Arc::new(None),
            &super::DatabaseStoreLevel::None,
            false,
        );

        let loadavg = monitoring_agent_lib::proc::ProcsLoadavg {
            loadavg1min: Some(1.0),
            loadavg5min: Some(2.0),
            loadavg10min: Some(3.1),
            current_running_processes: Some(1),
            total_number_of_processes: Some(10)
        };

        monitor.check_loadavg(&loadavg);

        let status = monitor.get_status();
        let status = status.lock().unwrap();
        assert_eq!(status.get("test").unwrap().status, super::Status::Error { message: "Load average check failed: 1min: Ok, 5min: Ok, 10min: Error { message: \"Load average 3.1 is greater than max load average 3\" }".to_string() } );
    }        
}