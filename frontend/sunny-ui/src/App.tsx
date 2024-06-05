import { useState } from 'react'
import {
  useQuery,
  useQueryClient,
  QueryClient,
  QueryClientProvider,
} from '@tanstack/react-query'
import sunnyLogo from './assets/sunny.svg'
import './App.css'

import { LineChart } from '@mui/x-charts/LineChart';
import Stack from '@mui/material/Stack';
import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs';
import { LocalizationProvider } from '@mui/x-date-pickers';
// import { DateTimePicker } from '@mui/x-date-pickers/DateTimePicker';
import { DatePicker } from '@mui/x-date-pickers/DatePicker';
// import { renderTimeViewClock } from '@mui/x-date-pickers/timeViewRenderers';
import Chip from '@mui/material/Chip';

import 'dayjs/locale/de';
import dayjs from 'dayjs';
import { ElectricalServices, Euro, Power, WbSunny } from '@mui/icons-material';


// Create the query client
const queryClient = new QueryClient()

// color settings
const colorPV = "#F4840B";
const colorFromGrid = "#FD5F3D";
const colorToGrid = "#9EDD61";
const colorPowerUsed = "#cdd0dc"; // TODO:

function App() {
  const now = dayjs();
  let startOfToday = dayjs().startOf('day');

  const [timeRange, setTimeRange] = useState(
    {
      start: startOfToday.unix() * 1000,
      end: now.unix() * 1000
    }
  )

  return (
    <QueryClientProvider client={queryClient}>
    <>
      <div>
        <img src={sunnyLogo} className="logo" alt="Sunny logo" />
      </div>
      <h1>Welcome to Sunny!</h1>

      <Stack spacing={4}>
      {/* <LocalizationProvider dateAdapter={AdapterDayjs} adapterLocale="de">
        <div className='row' style={{display: 'flex', justifyContent: 'space-around'}}>
        <DateTimePicker
          label="Start"
          viewRenderers={{
            hours: renderTimeViewClock,
            minutes: renderTimeViewClock,
            seconds: renderTimeViewClock,
          }}
          onChange={
            (value, context) => {
              if (value !== null) {
                setTimeRange({
                  start: value.unix() * 1000,
                  end: timeRange.end
                })
              }
            }
          }
        />
        <DateTimePicker
          label="End"
          viewRenderers={{
            hours: renderTimeViewClock,
            minutes: renderTimeViewClock,
            seconds: renderTimeViewClock,
          }}
          onChange={
            (value, context) => {
              if (value !== null) {
                setTimeRange({
                  start: timeRange.start,
                  end: value.unix() * 1000
                })
              }
            }
          }
        />
        </div>
    </LocalizationProvider> */}
    <LocalizationProvider dateAdapter={AdapterDayjs} adapterLocale="de">
    <div className='row' style={{display: 'flex', justifyContent: 'space-around'}}>
    <DatePicker label="Start"
      onChange={
        (value, context) => {
          if (value === null) return
          let ts = value.startOf('day').unix();
          setTimeRange({
            start: ts * 1000,
            end: timeRange.end
          })
        }
      }
      />
      <DatePicker label="End"
      onChange={
        (value, context) => {
          if (value === null) return
          let ts = value.endOf('day').unix();
          setTimeRange({
            start: timeRange.start,
            end: ts * 1000
          })
        }
      }
      />
    </div>
    </LocalizationProvider>

      <PowerValueChart
        timeRange={ timeRange }
      />

      <CurrentPowerValues
        timeRange={ timeRange }
      />

      </Stack>

    </>
    </QueryClientProvider>
  )

  // TODO: statistics & energy
}

export default App


function fetchDataAndStats(timeRange: { start: number, end: number }) {
  let url = `http://0.0.0.0:3000/values-with-stats/${timeRange.start}/${timeRange.end}`
  return fetch(url)
    .then((response) => response.json())
    .then((jsonResponse) => {
      return jsonResponse
    })
}

function CurrentPowerValues({ timeRange }: { timeRange: {start: number, end: number } }) {
  const queryClient = useQueryClient()
  // TODO: pull out the query and pass in data
  const query = useQuery({ queryKey: ['powerValuesWithStats', timeRange], queryFn: () => fetchDataAndStats(timeRange) })

  if (query.isError) {
    return (
      <div>Error fetching data. Are you connected to the Wifi?</div>
    )
  } else if (query.isPending) {
    return (
      <div>Loading data...</div>
    )
  } else if (query.isSuccess) {
    let data = query.data;
    let values = data.values;
    let [timestamp, current_values] = values[values.length - 1];
    let labelPV = `PV: ${valueToLabel(current_values.power_pv, 'W')}`;
    let labelUsed = `Usage: ${valueToLabel(current_values.power_used, 'W')}`;
    let labelToGrid = `To Grid: ${valueToLabel(current_values.power_to_grid, 'W')}`;
    let labelFromGrid = `From Grid: ${valueToLabel(current_values.power_from_grid, 'W')}`;

    return (
      <Stack spacing={1} direction="row" justifyContent="center">
        <Stack spacing={1}>
          <Chip icon={<WbSunny />} label={labelPV} sx={{ backgroundColor: colorPV + 'AA' }} variant="outlined" />
          <Chip icon={<Euro />} label={labelToGrid} style={{ backgroundColor: colorToGrid +'AA' }} />
        </Stack>
        <Stack spacing={1}>
          <Chip icon={<Power />} label={labelUsed} style={{ backgroundColor: colorPowerUsed +'AA' }} /> 
          <Chip icon={<ElectricalServices />} label={labelFromGrid} style={{ backgroundColor: colorFromGrid + 'AA' }} />
        </Stack>
      </Stack>
    )
  }

  return (
    <div>idk what happened here</div>
  )
}

function valueToLabel(value: number, unit: string) {
  if (value > 1000) {
    let val = (value / 1000.0).toPrecision(3);
    let u = 'k' + unit;
    return `${val} ${u}`
  } else {
    return `${value.toPrecision(3)} ${unit}`
  }
}


function PowerValueChart({ timeRange }: { timeRange: {start: number, end: number } }) {
  const queryClient = useQueryClient()
  // TODO: pull out the query and pass in data
  const query = useQuery({ queryKey: ['powerValuesWithStats', timeRange], queryFn: () => fetchDataAndStats(timeRange) })

  if (query.isError) {
    return (
      <div>Error fetching data. Are you connected to the Wifi?</div>
    )
  } else if (query.isPending) {
    return (
      <div>Loading data...</div>
    )
  } else if (query.isSuccess) {
    let data = query.data;
    let values = data.values;
    let unpackedValues = unpackValues(values);
    let timestamps = unpackedValues.timestamps;
    let powerValues = unpackedValues.powerValues;

  return (
    <LineChart
    xAxis={[{
      data: timestamps,
      dataKey: "timestamp",
      valueFormatter: (timestamp, context) => {
        let date = new Date(timestamp);
        return date.toLocaleString("de-at");
      },
      scaleType: "time"
    }]}
    series={[
      {
        data: powerValues.power_pv,
        showMark: false,
        color: colorPV,
        label: "Power PV"
      },
      {
        data: powerValues.power_used,
        showMark: false,
        label: "Power Used",
        color: colorPowerUsed
      },
      {
        data: powerValues.power_from_grid,
        showMark: false,
        label: "Power from Grid",
        color: colorFromGrid
      },
      {
        data: powerValues.power_to_grid,
        showMark: false,
        label: "Power into Grid",
        color: colorToGrid
      }
    ]}
    width={800}
    height={500}
    />
  )
  }

  return (
    <div>idk what happened here</div>
  )
}

function unpackValues(
  values: Array<
    [number,
      {
        power_pv: number,
        power_from_grid: number,
        power_to_grid: number,
        power_used: number
      }]>
  ){
  
  let timestamps: Array<number> = [];
  let powerValues: {
    power_pv: Array<number>,
    power_from_grid: Array<number>,
    power_to_grid: Array<number>,
    power_used: Array<number>
  } = {
    power_pv: [],
    power_from_grid: [],
    power_to_grid: [],
    power_used: []
  };

  values.forEach(
    (v) => {
      timestamps.push(v[0]);
      let powerVals = v[1];
      powerValues.power_pv.push(powerVals.power_pv / 1000.0);
      powerValues.power_from_grid.push(powerVals.power_from_grid / 1000.0);
      powerValues.power_to_grid.push(powerVals.power_to_grid / 1000.0);
      powerValues.power_used.push(powerVals.power_used / 1000.0);
    }
  )

  return {
    timestamps: timestamps,
    powerValues: powerValues
  }
}




// function TimeRangePicker({ onStartChange }: TimeRangeUpdate) {
//   return (
//     <LocalizationProvider dateAdapter={AdapterDayjs} adapterLocale="de">
//         <div className='row' style={{display: 'flex', justifyContent: 'space-around'}}>
//         <DateTimePicker
//           label="Start"
//           viewRenderers={{
//             hours: renderTimeViewClock,
//             minutes: renderTimeViewClock,
//             seconds: renderTimeViewClock,
//           }}
//           onChange={
//             (value, context) => { value ? onStartChange(value) : null }
//           }
//         />
//         <DateTimePicker
//           label="End"
//           viewRenderers={{
//             hours: renderTimeViewClock,
//             minutes: renderTimeViewClock,
//             seconds: renderTimeViewClock,
//           }}
//         />
//         </div>
//     </LocalizationProvider>
//   );
// }