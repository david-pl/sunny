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
import { DatePicker } from '@mui/x-date-pickers/DatePicker';
import Chip from '@mui/material/Chip';
import Card from '@mui/material/Card';
import { CardContent, IconButton } from '@mui/material';
import Typography from '@mui/material/Typography';

import 'dayjs/locale/de';
import dayjs from 'dayjs';
import { ElectricalServices, Euro, Power, WbSunny, ArrowBackIos, ArrowForwardIos } from '@mui/icons-material';

import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';

const forceLightTheme = createTheme({
  palette: {
    mode: 'light',  // otherwise, mobile will look bad
  },
});



// Create the query client
const queryClient = new QueryClient()

// color settings
const colorPV = "#F4840B";
const colorFromGrid = "#FD5F3D";
const colorToGrid = "#9EDD61";
const colorPowerUsed = "#cdd0dc";

function App() {
  return (
    <ThemeProvider theme={forceLightTheme}>
      <CssBaseline />
    <QueryClientProvider client={queryClient}>
      <>
        <div>
          <img src={sunnyLogo} className="logo" alt="Sunny logo" height="200"/>
        </div>
        <h2>Welcome to Sunny!</h2>

      <MainBody />

      </>
    </QueryClientProvider>
    </ThemeProvider>
  )

}

export default App


function MainBody() {
  let endOfToday = dayjs().endOf('day');
  let startOfToday = dayjs().startOf('day');

  const [timeRange, setTimeRange] = useState(
    {
      start: startOfToday.unix() * 1000,
      end: endOfToday.unix() * 1000
    }
  )

  const queryClient = useQueryClient()
  const query = useQuery({ queryKey: ['powerValuesWithStats', timeRange], queryFn: () => fetchDataAndStats(timeRange) })

  if (query.isError) {
    return (
      <div>Error fetching data.</div>
    )
  } else if (query.isPending) {
    return (
      <div>Loading data...</div>
    )
  } else if (query.isSuccess) {

  // query was successful
  let data = query.data;
  let values = data.values;
  let currentValues = values[values.length - 1][1];
  let energyValues = data.energy_kwh;
  let maxes = data.maxes;

  return (
    <Stack spacing={4}>
    <LocalizationProvider dateAdapter={AdapterDayjs} adapterLocale="de">
    <div className='row' style={{display: 'flex', justifyContent: 'space-around'}}>
    <IconButton
      onClick={
        // TODO: make this onClick properly update the DatePicker values
        () => {
          let oneDayInMillis = 60 * 60 * 24 * 1000;
          let newStart = timeRange.start - oneDayInMillis;
          let newEnd = timeRange.end - oneDayInMillis;
          setTimeRange({
            start: newStart,
            end: newEnd,
          })
        }
      }
    >
      <ArrowBackIos />
    </IconButton>
    <DatePicker label="Start" defaultValue={ dayjs.unix(timeRange.start / 1000) }
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
      <DatePicker label="End" defaultValue={ dayjs.unix(timeRange.end / 1000) }
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
      <IconButton
      onClick={
        () => {
          let oneDayInMillis = 60 * 60 * 24 * 1000;
          let newStart = timeRange.start + oneDayInMillis;
          let newEnd = timeRange.end + oneDayInMillis;
          setTimeRange({
            start: newStart,
            end: newEnd,
          })
        }
      }
      >
      <ArrowForwardIos />
    </IconButton>
    </div>
    </LocalizationProvider>

    <PowerValueChart
      values={ values }
    />

    <Stack spacing={5}>
      <CurrentPowerValues
        currentValues={ currentValues }
      />
      <EnergyValues
        currentValues={ energyValues }
      />
      <MaxPowerValues
        currentValues={ maxes }
      />
    </Stack>

  </Stack>
  );
}
}

function fetchDataAndStats(timeRange: { start: number, end: number }) {
  let url = `http://192.168.178.40:3000/values-with-stats/${timeRange.start}/${timeRange.end}`
  return fetch(url)
    .then((response) => response.json())
    .then((jsonResponse) => {
      return jsonResponse
    })
}

interface CurrentValues {
  currentValues: {
    power_pv: number,
    power_from_grid: number,
    power_to_grid: number,
    power_used: number
  }
}

function CurrentPowerValues({ currentValues }: CurrentValues) {
  return (
    <PowerValueCard 
      currentValues={ currentValues }
      unit={'W'}
      title={"Current Power Flow"}
    />
  )
}

function EnergyValues({ currentValues }: CurrentValues) {
  return (
    <PowerValueCard
      currentValues={ currentValues }
      unit={'kWh'}
      title={"Energy"}
    />
  )
}

function MaxPowerValues({ currentValues }: CurrentValues) {
  return (
    <PowerValueCard
      currentValues={ currentValues }
      unit={'W'}
      title={"Maximal Power"}
    />
  )
}

interface PowerValueCardValues {
  currentValues: {
    power_pv: number,
    power_from_grid: number,
    power_to_grid: number,
    power_used: number
  }
  unit: string
  title: string
}

function PowerValueCard({ currentValues, unit, title }: PowerValueCardValues ) {
  let labelPV = `PV: ${valueToLabel(currentValues.power_pv, unit)}`;
  let labelUsed = `Usage: ${valueToLabel(currentValues.power_used, unit)}`;
  let labelToGrid = `To Grid: ${valueToLabel(currentValues.power_to_grid, unit)}`;
  let labelFromGrid = `From Grid: ${valueToLabel(currentValues.power_from_grid, unit)}`;

  return (
    <Card variant="outlined">
    <CardContent>
    <Typography sx={{ fontSize: 14 }} color="text.secondary" gutterBottom>
      { title }
    </Typography>
    <Stack direction="row" justifyContent="center">
      <Stack>
        <Chip icon={<WbSunny />} label={labelPV} sx={{ backgroundColor: colorPV + 'AA' }} variant="outlined" />
        <Chip icon={<Euro />} label={labelToGrid} style={{ backgroundColor: colorToGrid +'AA' }} />
      </Stack>
      <Stack>
        <Chip icon={<Power />} label={labelUsed} style={{ backgroundColor: colorPowerUsed +'AA' }} /> 
        <Chip icon={<ElectricalServices />} label={labelFromGrid} style={{ backgroundColor: colorFromGrid + 'AA' }} />
      </Stack>
    </Stack>
    </CardContent>
    </Card>
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


interface ChartValues {
  values: Array<
  [number,
    {
      power_pv: number,
      power_from_grid: number,
      power_to_grid: number,
      power_used: number
    }]>
}

function PowerValueChart({ values }:  ChartValues  ) {
  let unpackedValues = unpackValues(values);
  let timestamps = unpackedValues.timestamps;
  let powerValues = unpackedValues.powerValues;

  // check if we need to display the dates (range over multiple days)
  let startDate = dayjs.unix(timestamps[0] / 1000);
  let endDate = dayjs.unix(timestamps[timestamps.length - 1] / 1000);
  let displayYear = startDate.year !== endDate.year;
  let displayDate = displayYear || (startDate.month !== endDate.month) || (startDate.day !== endDate.day);

  return (
    <LineChart
    xAxis={[{
      data: timestamps,
      dataKey: "timestamp",
      valueFormatter: (timestamp, context) => {
        let date = dayjs.unix(timestamp / 1e3);
        if (displayYear) {
          return date.format('DD.MM.YYYY H:mm')
        } else if (displayDate) {
          return date.format('DD.MM H:mm')
        } else {
          return date.format('H:mm')
        }
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
    width={600}
    height={700}
    />
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
