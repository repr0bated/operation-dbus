import { LineChart, Line, AreaChart, Area, BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

interface Props {
  data: { name: string; value: number }[];
  type?: 'line' | 'area' | 'bar';
  height?: number;
}

const theme = { stroke: '#e11d48', fill: '#e11d48', grid: '#27272a', text: '#a1a1aa' };

export function MetricChart({ data, type = 'line', height = 200 }: Props) {
  const common = { data, margin: { top: 5, right: 5, bottom: 5, left: 0 } };
  const axisProps = { stroke: theme.grid, tick: { fill: theme.text, fontSize: 11 } };

  return (
    <ResponsiveContainer width="100%" height={height}>
      {type === 'line' ? (
        <LineChart {...common}>
          <XAxis dataKey="name" {...axisProps} />
          <YAxis {...axisProps} />
          <Tooltip contentStyle={{ background: '#18181b', border: 'none' }} />
          <Line type="monotone" dataKey="value" stroke={theme.stroke} dot={false} />
        </LineChart>
      ) : type === 'area' ? (
        <AreaChart {...common}>
          <XAxis dataKey="name" {...axisProps} />
          <YAxis {...axisProps} />
          <Tooltip contentStyle={{ background: '#18181b', border: 'none' }} />
          <Area type="monotone" dataKey="value" stroke={theme.stroke} fill={theme.fill} fillOpacity={0.2} />
        </AreaChart>
      ) : (
        <BarChart {...common}>
          <XAxis dataKey="name" {...axisProps} />
          <YAxis {...axisProps} />
          <Tooltip contentStyle={{ background: '#18181b', border: 'none' }} />
          <Bar dataKey="value" fill={theme.fill} />
        </BarChart>
      )}
    </ResponsiveContainer>
  );
}
